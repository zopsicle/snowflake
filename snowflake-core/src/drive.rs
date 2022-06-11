use {
    crate::{
        action::{Action, ActionGraph, Input},
        label::ActionLabel,
        state::{ActionCacheEntry, State},
    },
    std::{collections::HashMap, os::unix::io::BorrowedFd},
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
}

/// The outcome of an attempt at building an action.
///
/// An action is considered *built*
/// if it is found in the action cache, or
/// if it is performed successfully and all outputs are able to be cached.
/// This type describes those scenarios, as well as possible failure cases.
#[allow(missing_docs)]
pub enum Outcome
{
    /// The action was built successfully.
    Success{
        cache_entry: ActionCacheEntry,
        cache_hit: bool,
    },

    /// Performing the action failed because of an error.
    Failed{error: BuildError},

    /// The action was skipped because a transitive dependency failed.
    Skipped{failed_dependency: ActionLabel},
}

/// Build all actions in an action graph.
pub fn drive<'a>(context: &Context, graph: &'a ActionGraph)
    -> Result<HashMap<&'a ActionLabel, Outcome>, DriveError>
{
    let linear = prepare(graph)?;

    let mut outcomes = HashMap::new();

    for (label, action, inputs) in linear {
        build(context, &mut outcomes, label, action, inputs);
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
fn build(
    context:  &Context,
    outcomes: &mut HashMap<&ActionLabel, Outcome>,
    label:    &ActionLabel,
    action:   &dyn Action,
    inputs:   &[Input],
)
{
    todo!()
}
