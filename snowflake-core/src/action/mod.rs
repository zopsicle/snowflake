//! Describing and performing actions.

pub use self::{graph::*, outputs::*};

use {
    snowflake_util::hash::Hash,
    std::{
        ffi::CString,
        os::unix::io::BorrowedFd,
        process::ExitStatusError,
        time::Duration,
    },
    thiserror::Error,
};

mod graph;
mod outputs;

/// Object-safe trait for actions.
pub trait Action
{
    /// The number of inputs to this action.
    fn inputs(&self) -> usize;

    /// The number of outputs of this action.
    fn outputs(&self) -> Outputs<usize>;

    /// Perform the action.
    ///
    /// This method takes paths to inputs and produces outputs.
    /// As a side-effect it writes to the [build log].
    /// It does not look up inputs in the cache or move outputs to the cache;
    /// that is part of *building* the action (see [`Outcome`]).
    ///
    /// The number of input paths must equal [`inputs`]
    /// and their order must match that of the inputs in [`ActionGraph`].
    ///
    /// [build log]: `Perform::build_log`
    /// [`Outcome`]: `crate::drive::Outcome`
    /// [`inputs`]: `Self::inputs`
    fn perform(&self, perform: &Perform, input_paths: &[InputPath]) -> Result;

    /// Compute the hash of the action.
    ///
    /// The number of input hashes must equal [`inputs`][`Self::inputs`]
    /// and their order must match that of the inputs in [`ActionGraph`].
    fn hash(&self, input_hashes: &[Hash]) -> Hash;
}

/// Extra methods for actions.
pub trait ActionExt
{
    /// Whether the action is a lint action.
    fn is_lint(&self) -> bool;
}

impl<T> ActionExt for T
    where T: Action + ?Sized
{
    fn is_lint(&self) -> bool
    {
        matches!(self.outputs(), Outputs::Lint)
    }
}

/// Environment in which an action is to be performed.
pub struct Perform<'a>
{
    /// File that contains the build log.
    pub build_log: BorrowedFd<'a>,

    /// Scratch directory which the action may use freely.
    pub scratch: BorrowedFd<'a>,
}

/// Path to an input and the directory to which it is relative.
#[allow(missing_docs)]
pub struct InputPath<'a>
{
    pub dirfd: BorrowedFd<'a>,
    pub path: CString,
}

/// Result of performing an action.
pub type Result =
    std::result::Result<Success, Error>;

/// Information about successfully performing an action.
///
/// Successfully performing an action might still cause the build to fail,
/// for example when some of the declared outputs do not actually exist.
#[derive(Debug)]
pub struct Success
{
    /// Pathnames of outputs produced by the action.
    ///
    /// The pathnames are relative to the [scratch directory].
    /// The number of outputs must equal [`Action::outputs`]
    /// and their indices must match those in [output labels].
    ///
    /// [scratch directory]: `Perform::scratch`
    /// [output labels]: `crate::label::ActionOutputLabel`
    pub output_paths: Vec<CString>,

    /// Whether warnings were emitted by the action.
    ///
    /// See the manual entry on warnings for
    /// the implications of setting this flag.
    pub warnings: bool,
}

/// Error returned during performing of an action.
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum Error
{
    #[error("Timeout after {0:?}")]
    Timeout(Duration),

    #[error("{0}")]
    ExitStatus(#[from] ExitStatusError),

    #[error("Unexpected error: {0}")]
    Unexpected(#[from] anyhow::Error),
}
