//! Identifying elements of a build.

use std::sync::Arc;

mod display;

/// Identifies a package.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PackageLabel
{
    pub segments: Arc<[Arc<str>]>,
}

/// Identifies a rule.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RuleLabel
{
    pub package: PackageLabel,
    pub rule: Arc<str>,
}

/// Identifies an action.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ActionLabel
{
    pub rule: RuleLabel,
    pub action: u32,
}

/// Identifies an output of a rule.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RuleOutputLabel
{
    pub rule: RuleLabel,
    pub output: Arc<str>,
}

/// Identifies an output of an action.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ActionOutputLabel
{
    pub action: ActionLabel,
    pub output: Arc<str>,
}
