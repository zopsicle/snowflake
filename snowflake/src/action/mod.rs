//! Describing and performing actions.

pub use self::graph::*;

use {crate::{hash::Blake3, label::ActionOutputLabel}, std::ffi::CString};

mod graph;

/// How to produce outputs given some configuration and inputs.
///
/// An action consists of configuration and inputs.
/// Configuration is all the parameters that do not depend on inputs.
/// Inputs are outputs of other actions that must be built first.
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
        inputs: Vec<ActionOutputLabel>,
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
            Self::RunCommand{inputs: _} => {
                hasher.put_u8(ACTION_TYPE_RUN_COMMAND);
            },
        }
    }

    /// The outputs of other actions that are inputs to this action.
    ///
    /// Inputs are yielded in arbitrary order and may include duplicates.
    pub fn inputs(&self) -> impl Iterator<Item=&ActionOutputLabel>
    {
        match self {
            Self::CreateSymbolicLink{..} => [].iter(),
            Self::WriteRegularFile{..} => [].iter(),
            Self::RunCommand{inputs} => inputs.iter(),
        }
    }
}
