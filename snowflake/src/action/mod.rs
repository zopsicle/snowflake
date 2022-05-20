pub use self::graph::*;

use {crate::label::ActionOutputLabel, std::ffi::CString};

mod graph;

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
