//! Identifying elements of a build.

mod display;

/// Identifies an action.
#[allow(missing_docs)]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ActionLabel
{
    pub action: usize,
}

/// Identifies an output of an action.
#[allow(missing_docs)]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ActionOutputLabel
{
    pub action: ActionLabel,
    pub output: usize,
}
