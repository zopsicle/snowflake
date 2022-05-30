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

/// Configuration and dependencies of an action.
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

/// Input to an action.
pub enum Input
{
    /// Dependency.
    Dependency(ActionOutputLabel),

    /// Source file.
    ///
    /// The path is interpreted to be relative to the source root.
    /// The hash must already be correct! It will not be verified.
    /// Including the hash in the input keeps [`hash_configuration`] pure.
    ///
    /// [`hash_configuration`]: `Action::hash_configuration`
    Source(PathBuf, Hash),
}

impl Action
{
    /// Hash of the configuration of the action.
    pub fn hash_configuration(&self, h: &mut Blake3)
    {
        // NOTE: See the manual chapter on avoiding hash collisions.

        const ACTION_TYPE_CREATE_SYMBOLIC_LINK: u8 = 0;
        const ACTION_TYPE_WRITE_REGULAR_FILE:   u8 = 1;
        const ACTION_TYPE_RUN_COMMAND:          u8 = 2;

        const INPUT_TYPE_DEPENDENCY: u8 = 0;
        const INPUT_TYPE_SOURCE:     u8 = 0;

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

                // The action graph is irrelevant to the configuration hash.
                // So we do not include which output a dependency refers to.
                // Sources are part of the configuration, so we include those.
                h.put_btree_map(inputs, |h, k, v| {
                    h.put_basename(k);
                    match v {
                        Input::Dependency(..) =>
                            h.put_u8(INPUT_TYPE_DEPENDENCY),
                        Input::Source(_, hash) => {
                            h.put_u8(INPUT_TYPE_SOURCE);
                            h.put_hash(*hash)
                        },
                    }
                });

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

    /// The dependencies of the action.
    ///
    /// The order in which the dependencies are yold
    /// corresponds to the order in which they are expected
    /// to be passed to [`perform`][`perform::perform`].
    pub fn dependencies(&self) -> impl Iterator<Item=&ActionOutputLabel>
    {
        match self {
            Self::CreateSymbolicLink{..} =>
                None.into_iter().flatten(),
            Self::WriteRegularFile{..} =>
                None.into_iter().flatten(),
            Self::RunCommand{inputs, ..} => {
                let iter =
                    inputs.values()
                    .filter_map(|i| match i {
                        Input::Dependency(d) => Some(d),
                        Input::Source(..) => None,
                    });
                Some(iter).into_iter().flatten()
            },
        }
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
