use {
    crate::label::{ActionLabel, ActionOutputLabel},
    super::Action,
    std::{collections::{HashMap, HashSet}, fmt},
};

/// Collection of actions and artifacts.
pub struct ActionGraph
{
    /// Actions to perform, in order implied by [inputs][`Action::inputs`].
    pub actions: HashMap<ActionLabel, Action>,

    /// Outputs made available to the user after building.
    pub artifacts: HashSet<ActionOutputLabel>,
}

impl ActionGraph
{
    /// Remove any actions not necessary to produce the artifacts.
    pub fn prune(&mut self)
    {
        // This implements a mark-and-sweep algorithm.

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
