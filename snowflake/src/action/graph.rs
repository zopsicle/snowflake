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
        let mut live = HashSet::new();

        // Lint actions are always considered live.
        live.extend(
            self.actions.iter()
            .filter(|a| a.1.is_lint())
            .map(|a| a.0.clone())
        );

        // Use mark-and-sweep to find other live actions.
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
        mark_recursively(&self.actions, &mut live, self.artifacts.iter());

        // Throw away all non-live actions.
        self.actions.retain(|k, _| live.contains(k));
    }
}

impl fmt::Display for ActionGraph
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        const FONTNAME:      &str = "monospace";
        const COLOR_ACTION:  &str = "/pastel28/1";
        const COLOR_LINT:    &str = "/pastel28/2";
        const COLOR_SPECIAL: &str = "/pastel28/3";

        write!(f, "digraph {{")?;
        write!(f, "node [fontname = {FONTNAME}, shape = box, style = filled];")?;
        write!(f, "edge [fontname = {FONTNAME}];")?;

        for (label, action) in &self.actions {
            let color = if action.is_lint() { COLOR_LINT } else { COLOR_ACTION };
            write!(f, "\"{label}\" [color = \"{color}\"];")?;
            for input in action.inputs() {
                write!(f,
                    "\"{}\" -> \"{}\" [label = {}];",
                    input.action, label, input.output)?;
            }
        }

        write!(f, "Artifacts [color = \"{COLOR_SPECIAL}\"];")?;
        for artifact in &self.artifacts {
            write!(f,
                "\"{}\" -> Artifacts [label = {}];",
                artifact.action, artifact.output)?;
        }

        write!(f, "}}")
    }
}
