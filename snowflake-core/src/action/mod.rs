//! Describing and performing actions.

pub use self::graph::*;

use {
    snowflake_util::hash::Hash,
    std::{
        os::unix::io::BorrowedFd,
        path::PathBuf,
        process::ExitStatusError,
        time::Duration,
    },
    thiserror::Error,
};

mod graph;

/// Object-safe trait for actions.
pub trait Action
{
    /// The number of inputs to this action.
    fn inputs(&self) -> usize;

    /// The number of outputs of this action.
    fn outputs(&self) -> usize;

    /// Perform the action.
    ///
    /// This method takes paths to inputs and produces outputs.
    /// As a side-effect it produces a build log (see [`Summary`]).
    /// It does not look up inputs in the cache or move outputs to the cache;
    /// these tasks are the responsibility of the caller.
    ///
    /// The number of input paths must equal [`inputs`][`Self::inputs`].
    fn perform(&self, perform: &Perform, input_paths: &[PathBuf]) -> Result;

    /// Compute the hash of the action.
    ///
    /// The number of input hashes must equal [`inputs`][`Self::inputs`].
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
        self.outputs() == 0
    }
}

/// Environment in which an action is to be performed.
pub struct Perform<'a>
{
    /// File that contains the build log.
    pub build_log: BorrowedFd<'a>,

    /// Source root, to which input paths are relative.
    pub source_root: BorrowedFd<'a>,

    /// Scratch directory which the action may use freely.
    pub scratch: BorrowedFd<'a>,
}

/// Result of performing an action.
pub type Result =
    std::result::Result<Summary, Error>;

/// Information about successfully performing an action.
///
/// Successfully performing an action might still cause the build to fail,
/// for example when some of the declared outputs do not actually exist.
#[derive(Debug)]
pub struct Summary
{
    /// Pathnames of outputs produced by the action.
    ///
    /// The pathnames are relative to the scratch directory.
    /// The number of outputs equals [`Action::outputs`].
    pub output_paths: Vec<PathBuf>,

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