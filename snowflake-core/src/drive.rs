use {
    crate::{
        action::ActionGraph,
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

pub fn drive(context: &Context, graph: &ActionGraph)
    -> Result<HashMap<ActionLabel, Outcome>, DriveError>
{
    let mut outcomes = HashMap::new();

    todo!();

    Ok(outcomes)
}
