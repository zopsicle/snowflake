use {
    crate::{
        action::{self, Action, ActionGraph, Input, InputPath, Perform},
        label::ActionLabel,
        state::{ActionCacheEntry, CacheOutputError, State},
    },
    os_ext::{O_RDWR, O_TMPFILE, cstr, openat},
    snowflake_util::hash::{Hash, hash_file_at},
    std::{
        borrow::Cow,
        collections::HashMap,
        io,
        os::unix::io::{AsFd, BorrowedFd},
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
    Io(#[from] io::Error),

    #[error("{0}")]
    Perform(#[from] action::Error),

    #[error("{0}")]
    CacheOutput(#[from] CacheOutputError),
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
    Failed{error: BuildError},

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
        Err(error) => Outcome::Failed{error},
    }
}

fn build_inner<'a>(
    context:  &Context,
    outcomes: &HashMap<&ActionLabel, Outcome<'a>>,
    action:   &dyn Action,
    inputs:   &'a [Input],
) -> Result<Outcome<'a>, BuildError>
{
    let mut input_paths: Vec<InputPath> =
        Vec::with_capacity(inputs.len());

    for input in inputs {
        match input {
            Input::Dependency(label) => {
                let outcome = outcomes.get(&label.action)
                    .expect("Action should have been built before");
                match outcome {
                    Outcome::Success{cache_entry, ..} => {
                        let hash = cache_entry.outputs.get(label.output)
                            .expect("Action refers to non-existent output");
                        let (dirfd, path) = context.state.cached_output(*hash)?;
                        let path = Cow::Owned(path);
                        input_paths.push(InputPath{dirfd, path});
                    },
                    Outcome::Failed{..} => {
                        let failed_dependency = &label.action;
                        return Ok(Outcome::Skipped{failed_dependency});
                    },
                    Outcome::Skipped{failed_dependency} =>
                        return Ok(Outcome::Skipped{failed_dependency}),
                }
            },
            Input::StaticFile(path) => {
                let dirfd = context.source_root;
                let path = Cow::Borrowed(path.as_ref());
                input_paths.push(InputPath{dirfd, path});
            },
        }
    }

    let input_hashes: Vec<Hash> =
        input_paths.iter()
        .map(|InputPath{dirfd, path}| hash_file_at(Some(*dirfd), path))
        .collect::<Result<_, _>>()?;

    let action_hash = action.hash(&input_hashes);

    if let Some(cache_entry) = context.state.cached_action(action_hash)? {
        return Ok(Outcome::Success{cache_entry, cache_hit: true});
    }

    let dir = context.state.as_fd();
    let build_log = openat(Some(dir), cstr!(b"."), O_TMPFILE | O_RDWR, 0o644)?;

    let scratch = context.state.new_scratch_dir()?;

    let perform = Perform{
        build_log: build_log.as_fd(),
        scratch: scratch.as_fd(),
    };

    let result = action.perform(&perform, &input_paths);

    let build_log = context.state.cache_build_log(build_log)?;
    // TODO: Include build log hash in outcome?

    match result {
        Ok(success) => {
            let outputs =
                (0 .. action.outputs().get())
                .map(|i| success.output_paths.get(i).expect("foo"))
                .map(|p| context.state.cache_output(Some(scratch.as_fd()), p))
                .collect::<Result<_, _>>()?;
            let warnings = success.warnings;
            let cache_entry = ActionCacheEntry{build_log, outputs, warnings};
            context.state.cache_action(action_hash, &cache_entry)?;
            Ok(Outcome::Success{cache_entry, cache_hit: false})
        },
        Err(error) =>
            Ok(Outcome::Failed{error: error.into()}),
    }
}
