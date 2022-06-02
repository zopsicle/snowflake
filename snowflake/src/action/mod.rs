//! Describing and performing actions.

pub use self::graph::*;

use {
    crate::{basename::Basename, hash::{Blake3, Hash}},
    self::perform_run_command::perform_run_command,
    anyhow::Context,
    os_ext::{O_CREAT, O_WRONLY, openat, symlinkat},
    regex::bytes::Regex,
    std::{
        ffi::CString,
        fs::File,
        io::Write,
        os::unix::io::BorrowedFd,
        path::PathBuf,
        process::ExitStatusError,
        sync::Arc,
        time::Duration,
    },
    thiserror::Error,
};

mod graph;
mod perform_run_command;

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

/// Action that creates a symbolic link.
pub struct CreateSymbolicLink
{
    /// The target of the symbolic link.
    pub target: CString,
}

impl Action for CreateSymbolicLink
{
    fn inputs(&self) -> usize
    {
        0
    }

    fn outputs(&self) -> usize
    {
        1
    }

    fn perform(&self, perform: &Perform, input_paths: &[PathBuf]) -> Result
    {
        debug_assert_eq!(input_paths.len(), 0);
        let output_path = PathBuf::from("output");
        symlinkat(&self.target, Some(perform.scratch), &output_path)
            .context("Create symbolic link")?;
        Ok(Summary{output_paths: vec![output_path], warnings: false})
    }

    fn hash(&self, input_hashes: &[Hash]) -> Hash
    {
        // NOTE: See the manual chapter on avoiding hash collisions.

        let Self{target} = self;

        debug_assert_eq!(input_hashes.len(), 0);

        let mut h = Blake3::new();
        h.put_str("CreateSymbolicLink");
        h.put_cstr(target);
        h.finalize()
    }
}

/// Action that writes a regular file.
pub struct WriteRegularFile
{
    /// The content of the regular file.
    pub content: Vec<u8>,

    /// Whether the executable bit is set
    /// in the mode of the regular file.
    pub executable: bool,
}

impl Action for WriteRegularFile
{
    fn inputs(&self) -> usize
    {
        0
    }

    fn outputs(&self) -> usize
    {
        1
    }

    fn perform(&self, perform: &Perform, input_paths: &[PathBuf]) -> Result
    {
        debug_assert_eq!(input_paths.len(), 0);
        let output_path = PathBuf::from("output");
        let flags = O_CREAT | O_WRONLY;
        let mode = if self.executable { 0o755 } else { 0o644 };
        let file = openat(Some(perform.scratch), &output_path, flags, mode)
            .context("Open regular file")?;
        File::from(file).write_all(&self.content)
            .context("Write regular file")?;
        Ok(Summary{output_paths: vec![output_path], warnings: false})
    }

    fn hash(&self, input_hashes: &[Hash]) -> Hash
    {
        // NOTE: See the manual chapter on avoiding hash collisions.

        let Self{content, executable} = self;

        debug_assert_eq!(input_hashes.len(), 0);

        let mut h = Blake3::new();
        h.put_str("WriteRegularFile");
        h.put_bytes(content);
        h.put_bool(*executable);
        h.finalize()
    }
}

/// Action that runs an arbitrary command in a container.
pub struct RunCommand
{
    /// What to call the inputs in the command's working directory.
    pub inputs: Vec<Arc<Basename>>,

    /// What the outputs are called in the command's working directory.
    pub outputs: Vec<Arc<Basename>>,

    /// Absolute path to the program to run.
    pub program: PathBuf,

    /// Arguments to the program.
    ///
    /// This should include the zeroth argument,
    /// which is normally equal to [`program`][`Self::program`].
    pub arguments: Vec<CString>,

    /// The environment variables to the program.
    ///
    /// This specifies the *exact* environment to the program.
    /// No extra environment variables are set by the
    /// [`perform`][`RunCommand::perform`] method.
    pub environment: Vec<CString>,

    /// How much time the program may spend.
    ///
    /// If the program spends more time than this,
    /// it is killed and the action fails.
    pub timeout: Duration,

    /// Regular expression that matches warnings in the build log.
    ///
    /// If [`None`], no warnings are assumed to have been emitted.
    pub warnings: Option<Regex>,
}

impl Action for RunCommand
{
    fn inputs(&self) -> usize
    {
        self.inputs.len()
    }

    fn outputs(&self) -> usize
    {
        self.outputs.len()
    }

    fn perform(&self, perform: &Perform, input_paths: &[PathBuf]) -> Result
    {
        perform_run_command(perform, self, input_paths)
    }

    fn hash(&self, input_hashes: &[Hash]) -> Hash
    {
        // NOTE: See the manual chapter on avoiding hash collisions.

        let Self{inputs, outputs, program, arguments,
                 environment, timeout, warnings} = self;

        debug_assert_eq!(input_hashes.len(), inputs.len());

        let mut h = Blake3::new();

        h.put_str("RunCommand");

        h.put_usize(inputs.len());
        for (basename, hash) in inputs.iter().zip(input_hashes) {
            h.put_basename(basename);
            h.put_hash(*hash);
        }

        h.put_slice(outputs, |h, o| h.put_basename(o));
        h.put_path(program);
        h.put_slice(arguments, |h, a| h.put_cstr(a));
        h.put_slice(environment, |h, e| h.put_cstr(e));

        // The timeout cannot affect the output of the action,
        // so there is no need to include it in the hash.
        let _ = timeout;

        h.put_bool(warnings.is_some());
        if let Some(warnings) = warnings {
            h.put_str(warnings.as_str());
        }

        h.finalize()
    }
}
