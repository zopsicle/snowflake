//! Describing and performing actions.

pub use self::graph::*;

use {crate::label::ActionOutputLabel, std::ffi::CString};

mod graph;

/// How to produce outputs given some configuration and inputs.
///
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
