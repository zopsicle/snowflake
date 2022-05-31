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
    pub fn hash<I>(&self, input_hashes: I) -> Hash
        where I: IntoIterator<Item=Hash>
    {
        let mut h = Blake3::new();
        self.hash_impl(&mut h, &mut input_hashes.into_iter());
        h.finalize()
    }

    fn hash_impl(&self, h: &mut Blake3,
                 input_hashes: &mut dyn Iterator<Item=Hash>)
    {
        // NOTE: See the manual chapter on avoiding hash collisions.

        const ACTION_TYPE_CREATE_SYMBOLIC_LINK: u8 = 0;
        const ACTION_TYPE_WRITE_REGULAR_FILE:   u8 = 1;
        const ACTION_TYPE_RUN_COMMAND:          u8 = 2;

        match self {

            Self::CreateSymbolicLink{target} => {
                h.put_u8(ACTION_TYPE_CREATE_SYMBOLIC_LINK);
                h.put_cstr(target);
            },

            Self::WriteRegularFile{content, executable} => {
                h.put_u8(ACTION_TYPE_WRITE_REGULAR_FILE);
                h.put_bytes(content);
                h.put_bool(*executable);
            },

            Self::RunCommand{inputs, outputs, program, arguments,
                             environment, timeout, warnings} => {
                h.put_u8(ACTION_TYPE_RUN_COMMAND);

                h.put_btree_map(inputs, |h, k, v| {
                    h.put_basename(k);
                    h.put_hash(input_hashes.next()
                        .expect("Not enough inputs for computing action hash"));
                    // Whether it's a dependency or a static file
                    // cannot be observabled by the action,
                    // so no need to include that in the hash.
                    let _ = v;
                    h
                });
                assert!(input_hashes.next().is_none(),
                        "Too many inputs when computing action hash");

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
