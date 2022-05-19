use {
    crate::label::{ActionLabel, ActionOutputLabel},
    std::{collections::{HashMap, HashSet}, ffi::CString, fmt},
};

pub struct ActionGraph
{
    pub actions: HashMap<ActionLabel, Action>,
    pub artifacts: HashSet<ActionOutputLabel>,
}

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
    pub fn inputs(&self) -> impl '_ + Iterator<Item=ActionOutputLabel>
    {
        match self {
            Self::CreateSymbolicLink{..} => [].iter().cloned(),
            Self::WriteRegularFile{..} => [].iter().cloned(),
            Self::RunCommand{inputs} => inputs.iter().cloned(),
        }
    }
}

impl fmt::Display for ActionGraph
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        write!(f, "digraph {{")?;

        for (label, action) in &self.actions {
            write!(f, "\"{}\" [shape = \"box\"];", label)?;
            for input in action.inputs() {
                write!(f,
                    "\"{}\" -> \"{}\" [label = \"{}\"];",
                    input.action, label, input.output)?;
            }
        }

        write!(f, "\"«artifacts»\" [shape = \"box\"];")?;
        for artifact in &self.artifacts {
            write!(f,
                "\"{}\" -> \"«artifacts»\" [label = \"{}\"];",
                artifact.action, artifact.output)?;
        }

        write!(f, "}}")
    }
}
