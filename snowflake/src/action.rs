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

impl ActionGraph
{
    /// Remove any actions not necessary to produce the artifacts.
    pub fn mark_and_sweep(&mut self)
    {
        fn mark_recursively<'a>(
            graph: &HashMap<ActionLabel, Action>,
            live: &mut HashSet<ActionLabel>,
            outputs: impl Iterator<Item=&'a ActionOutputLabel>,
        )
        {
            for ActionOutputLabel{action, ..} in outputs {
                if !live.insert(action.clone()) {
                    continue;
                }
                let action = graph.get(action)
                    .expect("Action graph is missing action");
                mark_recursively(graph, live, action.inputs());
            }
        }

        let mut live = HashSet::new();
        mark_recursively(&self.actions, &mut live, self.artifacts.iter());
        self.actions.retain(|k, _| live.contains(k));
    }
}

impl Action
{
    pub fn inputs(&self) -> impl Iterator<Item=&ActionOutputLabel>
    {
        match self {
            Self::CreateSymbolicLink{..} => [].iter(),
            Self::WriteRegularFile{..} => [].iter(),
            Self::RunCommand{inputs} => inputs.iter(),
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
