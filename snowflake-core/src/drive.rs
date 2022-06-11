use {
    crate::{
        action::{self, Action, ActionGraph, Input, InputPath, Perform, Success},
        label::ActionLabel,
        state::{ActionCacheEntry, CacheOutputError, State},
    },
    anyhow::{Context as _},
    os_ext::{O_RDWR, O_TMPFILE, cstr, openat},
    snowflake_util::hash::{Hash, hash_file_at},
    std::{
        borrow::Cow,
        collections::HashMap,
        os::unix::io::{AsFd, BorrowedFd, OwnedFd},
    },
    thiserror::Error,
};

pub struct Context<'a>
{
    pub state: &'a State,
    pub source_root: BorrowedFd<'a>,
}

#[derive(Debug, Error)]
pub enum DriveError
{
    #[error("There are actions that cyclically depend on each other")]
    // TODO: Which actions?
    CyclicDependency,

    #[error("There is an action that depends on a missing action")]
    // TODO: Which actions?
    DanglingDependency,
}

#[derive(Debug, Error)]
pub enum BuildError
{
    #[error("{0}")]
    Perform(#[from] action::Error),

    #[error("{0}")]
    CacheOutput(#[from] CacheOutputError),

    #[error("Unexpected error: {0}")]
    Unexpected(#[from] anyhow::Error),
}

/// The outcome of an attempt at building an action.
///
/// An action is considered *built*
/// if it is found in the action cache, or
/// if it is performed successfully and all outputs are able to be cached.
/// This type describes those scenarios, as well as possible failure cases.
#[allow(missing_docs)]
#[derive(Debug)]
pub enum Outcome<'a>
{
    /// The action was built successfully.
    Success{
        cache_entry: ActionCacheEntry,
        cache_hit: bool,
    },

    /// Performing the action failed because of an error.
    Failed{
        build_log: Option<Hash>,
        error: BuildError,
    },

    /// The action was skipped because a transitive dependency failed.
    Skipped{failed_dependency: &'a ActionLabel},
}

/// Build all actions in an action graph.
pub fn drive<'a>(context: &Context, graph: &'a ActionGraph)
    -> Result<HashMap<&'a ActionLabel, Outcome<'a>>, DriveError>
{
    let linear = prepare(graph)?;

    let mut outcomes = HashMap::new();

    for (label, action, inputs) in linear {
        let outcome = build(context, &outcomes, action, inputs);
        outcomes.insert(label, outcome);
    }

    Ok(outcomes)
}

/// Topologically sort the action graph.
fn prepare(graph: &ActionGraph)
    -> Result<Vec<(&ActionLabel, &dyn Action, &[Input])>, DriveError>
{
    fn toposort<'a>(
        linear: &mut Vec<(&'a ActionLabel, &'a dyn Action, &'a [Input])>,
        // The state table keeps track of visited actions.
        // An false entry means the action is currently being visited.
        // A true entry means the action was visited in the past.
        // These states are used for detecting cycles
        // and avoiding duplicates respectively.
        state: &mut HashMap<&'a ActionLabel, bool>,
        graph: &'a ActionGraph,
        label: &'a ActionLabel,
    ) -> Result<(), DriveError>
    {
        match state.get(label) {
            Some(false) => Err(DriveError::CyclicDependency),
            Some(true)  => Ok(()),
            None =>
                if let Some((action, inputs)) = graph.actions.get(label) {
                    state.insert(label, false);
                    for input in inputs.iter().flat_map(Input::dependency) {
                        toposort(linear, state, graph, &input.action)?;
                    }
                    state.insert(label, true);
                    linear.push((label, &**action, inputs));
                    Ok(())
                } else {
                    Err(DriveError::DanglingDependency)
                },
        }
    }

    let mut linear = Vec::new();
    let mut state = HashMap::new();
    for action in graph.actions.keys() {
        toposort(&mut linear, &mut state, graph, action)?;
    }
    Ok(linear)
}

/// Build an action.
fn build<'a>(
    context:  &Context,
    outcomes: &HashMap<&ActionLabel, Outcome<'a>>,
    action:   &dyn Action,
    inputs:   &'a [Input],
) -> Outcome<'a>
{
    match build_inner(context, outcomes, action, inputs) {
        Ok(outcome) => outcome,
        Err(error) => Outcome::Failed{build_log: None, error},
    }
}

fn build_inner<'a>(
    context:  &Context,
    outcomes: &HashMap<&ActionLabel, Outcome<'a>>,
    action:   &dyn Action,
    inputs:   &'a [Input],
) -> Result<Outcome<'a>, BuildError>
{
    let input_paths = collect_input_paths(context, outcomes, inputs)?;
    let input_paths = match input_paths {
        Ok(input_paths) => input_paths,
        Err(fd) => return Ok(Outcome::Skipped{failed_dependency: fd}),
    };
    let action_hash = compute_action_hash(action, &input_paths)?;
    if let Some(cache_entry) = check_action_cache(context, action_hash)? {
        return Ok(Outcome::Success{cache_entry, cache_hit: true});
    }
    let build_log = create_build_log(context)?;
    let scratch = context.state.new_scratch_dir()                               .with_context(|| "Create scratch directory")?;
    let result = perform_action(action, &input_paths, &build_log, &scratch);
    let build_log = context.state.cache_build_log(build_log)                    .with_context(|| "Move build log to output cache")?;
    match result {
        Ok(success) => cache_action(context, action, action_hash, build_log, &scratch, &success),
        Err(error) => Ok(Outcome::Failed{build_log: Some(build_log), error: error.into()}),
    }
}

/// Compute the path of each input.
///
/// If inputs are missing due to unfortunate outcomes of dependencies,
/// this function returns early with the dependency that failed.
fn collect_input_paths<'a, 'b>(
    context:  &'a Context,
    outcomes: &HashMap<&ActionLabel, Outcome<'b>>,
    inputs:   &'b [Input],
) -> Result<Result<Vec<InputPath<'a, 'b>>, &'b ActionLabel>, BuildError>
{
    let mut input_paths = Vec::with_capacity(inputs.len());

    for input in inputs {
        match input {
            Input::Dependency(label) => {
                let outcome = outcomes.get(&label.action)
                    .expect("Action should have been built before");
                match outcome {
                    Outcome::Success{cache_entry, ..} => {
                        let hash = cache_entry.outputs.get(label.output)
                            .expect("Action refers to non-existent output");
                        let (dirfd, path) = context.state.cached_output(*hash)  .with_context(|| "Retrieve dependency from output cache")?;
                        let path = Cow::Owned(path);
                        input_paths.push(InputPath{dirfd, path});
                    },
                    Outcome::Failed{..} =>
                        return Ok(Err(&label.action)),
                    Outcome::Skipped{failed_dependency} =>
                        return Ok(Err(failed_dependency)),
                }
            },
            Input::StaticFile(path) => {
                let dirfd = context.source_root;
                let path = Cow::Borrowed(path.as_ref());
                input_paths.push(InputPath{dirfd, path});
            },
        }
    }

    Ok(Ok(input_paths))
}

/// Compute the hash of an action, which is its key into the action cache.
fn compute_action_hash(action: &dyn Action, input_paths: &[InputPath])
    -> Result<Hash, BuildError>
{
    let mut input_hashes = Vec::with_capacity(input_paths.len());

    for InputPath{dirfd, path} in input_paths {
        let hash = hash_file_at(Some(*dirfd), path)                             .with_context(|| "Compute hash of input")?;
        input_hashes.push(hash);
    }

    Ok(action.hash(&input_hashes))
}

/// Look up the action in the action cache, in order to skip the build.
fn check_action_cache(context: &Context, action_hash: Hash)
    -> Result<Option<ActionCacheEntry>, BuildError>
{
    let cache_entry = context.state.cached_action(action_hash)                  .with_context(|| "Look up action in action cache")?;
    Ok(cache_entry)
}

/// Create the file that will store the build log.
fn create_build_log(context: &Context) -> Result<OwnedFd, BuildError>
{
    let state_dir = context.state.as_fd();
    let file = openat(Some(state_dir), cstr!(b"."), O_TMPFILE | O_RDWR, 0o644)  .with_context(|| "Create build log")?;
    Ok(file)
}

/// Perform the action.
fn perform_action(
    action: &dyn Action,
    input_paths: &[InputPath],
    build_log: &OwnedFd,
    scratch: &OwnedFd,
) -> action::Result
{
    let perform = Perform{
        build_log: build_log.as_fd(),
        scratch: scratch.as_fd(),
    };
    action.perform(&perform, input_paths)
}

/// Insert the outputs and action into the caches.
fn cache_action<'a>(
    context:     &Context,
    action:      &dyn Action,
    action_hash: Hash,
    build_log:   Hash,
    scratch:     &OwnedFd,
    success:     &Success,
) -> Result<Outcome<'a>, BuildError>
{
    let outputs = cache_outputs(context, action, scratch, success)?;
    let warnings = success.warnings;
    let cache_entry = ActionCacheEntry{build_log, outputs, warnings};
    context.state.cache_action(action_hash, &cache_entry)                       .with_context(|| "Insert action into action cache")?;
    Ok(Outcome::Success{cache_entry, cache_hit: false})
}

/// Move every output to the output cache and return their hashes.
fn cache_outputs(
    context: &Context,
    action:  &dyn Action,
    scratch: &OwnedFd,
    success: &Success,
) -> Result<Vec<Hash>, BuildError>
{
    let scratch = scratch.as_fd();
    let count = action.outputs().get();

    // Can only be triggered by a fauly implementation of Action::perform.
    // So there is no need to return a user-facing error for this.
    assert_eq!(success.output_paths.len(), count,
        "Action must produce as many outputs as declared");

    let mut output_hashes = Vec::with_capacity(count);

    for output_path in &success.output_paths {
        // Outputs are placed by the action in the scratch directory.
        // And the output path is relative to the scratch directory.
        let hash = context.state.cache_output(Some(scratch), output_path)?;
        output_hashes.push(hash);
    }

    Ok(output_hashes)
}
