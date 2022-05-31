//! Describing and performing actions.

pub use self::graph::*;

use {
    crate::{basename::Basename, hash::{Blake3, Hash}, label::ActionOutputLabel},
    regex::bytes::Regex,
    std::{
        collections::BTreeMap,
        ffi::CString,
        path::PathBuf,
        sync::Arc,
        time::Duration,
    },
};

pub mod perform;

mod graph;

/// Any type of action.
#[allow(missing_docs)]
pub enum Action
{
    CreateSymbolicLink{
        target: CString,
    },

    WriteRegularFile{
        content: Vec<u8>,
        executable: bool,
    },

    RunCommand{
        // Using a B-tree ensures a stable ordering,
        // which is important for the configuration hash.
        inputs: BTreeMap<Arc<Basename>, Input>,
        outputs: Vec<Arc<Basename>>,
        program: PathBuf,
        arguments: Vec<CString>,
        environment: Vec<CString>,
        timeout: Duration,
        warnings: Option<Regex>,
    },
}

/// Any type of input.
pub enum Input
{
    /// Dependency.
    Dependency(ActionOutputLabel),

    /// Static file.
    ///
    /// The path is interpreted to be relative to the source root.
    StaticFile(PathBuf),
}

impl Action
{
    /// Compute the hash of the action.
    ///
    /// The hashes of the inputs must be given in
    /// the same order as [`inputs`][`Self::inputs`].
    pub fn hash<I>(&self, input_hashes: &[Hash]) -> Hash
    {
        // NOTE: See the manual chapter on avoiding hash collisions.

        let mut h = Blake3::new();

        const ACTION_TYPE_CREATE_SYMBOLIC_LINK: u8 = 0;
        const ACTION_TYPE_WRITE_REGULAR_FILE:   u8 = 1;
        const ACTION_TYPE_RUN_COMMAND:          u8 = 2;

        match self {

            Self::CreateSymbolicLink{target} => {
                debug_assert_eq!(input_hashes.len(), 0);
                h.put_u8(ACTION_TYPE_CREATE_SYMBOLIC_LINK);
                h.put_cstr(target);
            },

            Self::WriteRegularFile{content, executable} => {
                debug_assert_eq!(input_hashes.len(), 0);
                h.put_u8(ACTION_TYPE_WRITE_REGULAR_FILE);
                h.put_bytes(content);
                h.put_bool(*executable);
            },

            Self::RunCommand{inputs, outputs, program, arguments,
                             environment, timeout, warnings} => {
                debug_assert_eq!(input_hashes.len(), inputs.len());
                h.put_u8(ACTION_TYPE_RUN_COMMAND);

                h.put_usize(inputs.len());
                for (basename, hash) in inputs.keys().zip(input_hashes) {
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
            },

        }

        h.finalize()
    }

    /// The inputs of the action.
    ///
    /// The order in which the inputs are yold by the iterator
    /// corresponds to the order in which they are expected
    /// to be passed to [`hash`][`Self::hash`],
    /// [`perform`][`perform::perform`], and other places.
    pub fn inputs(&self) -> impl Iterator<Item=&Input>
    {
        static EMPTY: BTreeMap<Arc<Basename>, Input> = BTreeMap::new();
        match self {
            Self::CreateSymbolicLink{..} => EMPTY.values(),
            Self::WriteRegularFile{..}   => EMPTY.values(),
            Self::RunCommand{inputs, ..} => inputs.values()
        }
    }

    /// The dependencies of the action.
    pub fn dependencies(&self) -> impl Iterator<Item=&ActionOutputLabel>
    {
        self.inputs().filter_map(|i| {
            match i {
                Input::Dependency(d) => Some(d),
                Input::StaticFile(..) => None,
            }
        })
    }

    /// The number of outputs of this action.
    pub fn outputs(&self) -> usize
    {
        match self {
            Self::CreateSymbolicLink{..}  => 1,
            Self::WriteRegularFile{..}    => 1,
            Self::RunCommand{outputs, ..} => outputs.len(),
        }
    }

    /// Whether this action is a lint action.
    pub fn is_lint_action(&self) -> bool
    {
        self.outputs() == 0
    }
}
