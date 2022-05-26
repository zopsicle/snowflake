use {
    super::Action,
    os_ext::{O_CREAT, O_WRONLY, openat, symlinkat},
    std::{
        ffi::CStr,
        fs::File,
        io::{self, Write},
        os::unix::io::BorrowedFd,
        path::Path,
    },
    thiserror::Error,
};

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
        Action::RunCommand{inputs, outputs, warnings} =>
            perform_run_command(perform),
    }
}

fn perform_create_symbolic_link(perform: &Perform, target: &CStr) -> Result
{
    symlinkat(target, Some(perform.outputs), Path::new("0"))?;
    Ok(Summary{warnings: false})
}

fn perform_write_regular_file(
    perform: &Perform,
    content: &[u8],
    executable: bool,
) -> Result
{
    let flags = O_CREAT | O_WRONLY;
    let mode = if executable { 0o755 } else { 0o644 };
    let file = openat(Some(perform.outputs), Path::new("0"), flags, mode)?;
    File::from(file).write_all(content)?;
    Ok(Summary{warnings: false})
}

use self::run_command::perform_run_command;
mod run_command;
