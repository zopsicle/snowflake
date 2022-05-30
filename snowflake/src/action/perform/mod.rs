//! Unified interface for performing different types of actions.
//!
//! The [`perform`] function takes inputs and produces outputs,
//! according to the description of the action given to it.
//! As a side-effect it produces a build log (see [`Summary`]).
//! It does not look up inputs in the cache or move outputs to the cache;
//! these tasks are the responsibility of the caller.

use {
    super::Action,
    os_ext::{O_CREAT, O_WRONLY, openat, symlinkat},
    std::{
        ffi::{CStr, NulError},
        fs::File,
        io::{self, Write},
        os::unix::io::BorrowedFd,
        path::PathBuf,
        process::ExitStatusError,
    },
    thiserror::Error,
};

/// Environment in which an action is to be performed.
pub struct Perform<'a>
{
    /// File that contains the build log.
    pub build_log: BorrowedFd<'a>,

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
pub struct Summary
{
    /// Pathnames of outputs produced by the action.
    ///
    /// The pathnames are relative to the scratch directory.
    /// The number of outputs equals [`Action::outputs`].
    pub outputs: Vec<PathBuf>,

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
    #[error("{0}")]
    Io(#[from] io::Error),

    #[error("{0}")]
    Nul(#[from] NulError),

    #[error("Container setup: {1}: {0}")]
    ContainerSetup(io::Error, String),

    #[error("Timeout")]
    Timeout,

    #[error("{0}")]
    ExitStatus(#[from] ExitStatusError),
}

/// Perform an action.
///
/// See the [module documentation][`self`] for more information.
pub fn perform(perform: &Perform, action: &Action) -> Result
{
    match action {
        Action::CreateSymbolicLink{target} =>
            perform_create_symbolic_link(perform, target),
        Action::WriteRegularFile{content, executable} =>
            perform_write_regular_file(perform, content, *executable),
        Action::RunCommand{inputs, outputs, program, arguments,
                           environment, timeout, warnings} =>
            perform_run_command(perform, outputs, program, arguments,
                                environment, *timeout),
    }
}

fn perform_create_symbolic_link(perform: &Perform, target: &CStr) -> Result
{
    let output = PathBuf::from("output");
    symlinkat(target, Some(perform.scratch), &output)?;
    Ok(Summary{outputs: vec![output], warnings: false})
}

fn perform_write_regular_file(
    perform: &Perform,
    content: &[u8],
    executable: bool,
) -> Result
{
    let output = PathBuf::from("output");
    let flags = O_CREAT | O_WRONLY;
    let mode = if executable { 0o755 } else { 0o644 };
    let file = openat(Some(perform.scratch), &output, flags, mode)?;
    File::from(file).write_all(content)?;
    Ok(Summary{outputs: vec![output], warnings: false})
}

use self::run_command::perform_run_command;
mod run_command;
