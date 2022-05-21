//! Identifying elements of a build.

use {std::{ffi::OsStr, os::unix::ffi::OsStrExt, sync::Arc}, thiserror::Error};

mod display;

/// Identifies a package.
#[allow(missing_docs)]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PackageLabel
{
    pub segments: Arc<[Basename]>,
}

/// Identifies a rule.
#[allow(missing_docs)]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RuleLabel
{
    pub package: PackageLabel,
    pub rule: Arc<str>,
}

/// Identifies an action.
#[allow(missing_docs)]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ActionLabel
{
    pub rule: RuleLabel,
    pub action: u32,
}

/// Identifies an output of a rule.
#[allow(missing_docs)]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct RuleOutputLabel
{
    pub rule: RuleLabel,
    pub output: Basename,
}

/// Identifies an output of an action.
#[allow(missing_docs)]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ActionOutputLabel
{
    pub action: ActionLabel,
    pub output: u32,
}

/// Name of a package segment or rule output.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Basename
{
    // INVARIANT: See the restrictions `new` imposes.
    inner: Arc<OsStr>,
}

/// Returned when a basename could not be validated.
///
/// See [`Basename::new`] for the restrictions on basenames.
#[derive(Debug, Error)]
#[error("Basename is empty, `.`, or `..`, or contains `/`")]
pub struct BasenameError;

impl Basename
{
    /// Create a basename from an OS string.
    ///
    /// Returns an error if the basename is invalid.
    /// A basename is invalid if it is empty, `.`, or `..`, or contains `/`.
    pub fn new(inner: Arc<OsStr>) -> Result<Self, BasenameError>
    {
        let bytes = inner.as_bytes();

        if matches!(bytes, b"" | b"." | b"..") {
            return Err(BasenameError);
        }

        if bytes.contains(&b'/') {
            return Err(BasenameError);
        }

        Ok(Self{inner})
    }
}
