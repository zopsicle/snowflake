use {
    self::{create_symbolic_link::*, run_command::*, write_regular_file::*},
    super::Action,
    std::{io, os::unix::io::BorrowedFd},
    thiserror::Error,
};

mod create_symbolic_link;
mod run_command;
mod write_regular_file;

/// Information needed to perform an action.
pub struct Perform<'a>
{
    /// Log file.
    ///
    /// The action may write arbitrary text to this file.
    /// If performing the action fails with an error,
    /// the error is also appended to the log.
    pub log: BorrowedFd<'a>,

    /// Output directory.
    ///
    /// After successfully performing an action,
    /// all outputs of the action exist in this directory.
    /// The output files are named 0, 1, …, _n_ &minus; 1
    /// where _n_ is [`Action::outputs`].
    pub outputs: BorrowedFd<'a>,
}

/// Result of performing an action.
pub type Result =
    std::result::Result<Summary, Error>;

/// Information about successfully performing an action.
pub struct Summary
{
    /// Whether warnings were emitted.
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
}

/// Perform an action.
pub fn perform(perform: &Perform, action: &Action) -> Result
{
    match action {
        Action::CreateSymbolicLink{target} =>
            perform_create_symbolic_link(perform, target),
        Action::WriteRegularFile{content, executable} =>
            perform_write_regular_file(perform, content, *executable),
        Action::RunCommand{inputs, outputs} =>
            perform_run_command(perform),
    }
}
