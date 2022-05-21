//! Describing and performing actions.

pub use self::graph::*;

use {
    crate::{basename::Basename, hash::Blake3, label::ActionOutputLabel},
    std::{collections::BTreeMap, ffi::CString, num::NonZeroU32, sync::Arc},
};

mod graph;

/// How to produce outputs given some configuration and inputs.
///
/// An action consists of configuration and action graph structure.
/// Configuration is static information; it does not change
/// based on the output of the action's dependencies.
/// Inputs are outputs of other actions that must be built first.
/// Output labels form the edges of the [action graph][`ActionGraph`].
/// The different types of actions and their parameters
/// are documented in detail in the manual.
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
        inputs: BTreeMap<Arc<Basename>, ActionOutputLabel>,
        outputs: Vec<Arc<Basename>>,
    },
}

impl Action
{
    /// Hash of the configuration of the action.
    pub fn hash_configuration(&self, hasher: &mut Blake3)
    {
        // NOTE: See the manual chapter on avoiding hash collisions.
        const ACTION_TYPE_CREATE_SYMBOLIC_LINK: u8 = 0;
        const ACTION_TYPE_WRITE_REGULAR_FILE:   u8 = 1;
        const ACTION_TYPE_RUN_COMMAND:          u8 = 2;
        match self {
            Self::CreateSymbolicLink{target} => {
                hasher.put_u8(ACTION_TYPE_CREATE_SYMBOLIC_LINK);
                hasher.put_cstr(target);
            },
            Self::WriteRegularFile{content, executable} => {
                hasher.put_u8(ACTION_TYPE_WRITE_REGULAR_FILE);
                hasher.put_bytes(content);
                hasher.put_bool(*executable);
            },
            Self::RunCommand{inputs, outputs} => {
                hasher.put_u8(ACTION_TYPE_RUN_COMMAND);

                // The action graph structure is irrelevant to the hash.
                // So we only include the names of the inputs,
                // and not the files they represent in the action graph.
                hasher.put_usize(inputs.len());
                for input in inputs.keys() {
                    hasher.put_basename(input);
                }

                hasher.put_usize(outputs.len());
                for output in outputs {
                    hasher.put_basename(output);
                }
            },
        }
    }

    /// The outputs of other actions that are inputs to this action.
    ///
    /// Inputs are yielded in arbitrary order and may include duplicates.
    pub fn inputs(&self) -> impl Iterator<Item=&ActionOutputLabel>
    {
        match self {
            Self::CreateSymbolicLink{..} =>
                None.into_iter().flatten(),
            Self::WriteRegularFile{..} =>
                None.into_iter().flatten(),
            Self::RunCommand{inputs, ..} =>
                Some(inputs.values()).into_iter().flatten(),
        }
    }

    /// The number of outputs of this action.
    pub fn outputs(&self) -> NonZeroU32
    {
        match self {
            Self::CreateSymbolicLink{..} =>
                NonZeroU32::new(1).unwrap(),
            Self::WriteRegularFile{..} =>
                NonZeroU32::new(1).unwrap(),
            Self::RunCommand{outputs, ..} => {
                let n = outputs.len().try_into()
                    .expect("Action has too many outputs");
                NonZeroU32::new(n).expect("Action has no outputs")
            },
        }
    }
}
