use {
    crate::label::{ActionLabel, ActionOutputLabel},
    super::{Action, ActionExt},
    std::{collections::{HashMap, HashSet}, fmt, path::PathBuf},
};

/// Action graph encoded as an adjacency list.
///
/// The vertices of the graph are stored indexed by action label.
/// The edges of the graph are encoded by the dependency sets of the actions.
pub struct ActionGraph
{
    /// Actions to perform, in the order implied by their dependency graph.
    pub actions: HashMap<ActionLabel, (Box<dyn Action>, Vec<Input>)>,

    /// Artifacts of the requested build.
    pub artifacts: HashSet<ActionOutputLabel>,
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

impl Input
{
    /// If this input is a dependency, the dependency.
    pub fn dependency(&self) -> Option<&ActionOutputLabel>
    {
        match self {
            Self::Dependency(d) => Some(d),
            Self::StaticFile(..) => None,
        }
    }
}

impl ActionGraph
{
    /// Remove any actions that do not need to be performed.
    ///
    /// Actions that do not need to be performed are non-lint actions
    /// which are not transitively depended upon by the artifact set.
    pub fn prune(&mut self)
    {
        let mut live = HashSet::new();

        // Lint actions are always considered live.
        live.extend(
            self.actions.iter()
            .filter(|a| a.1.0.is_lint())
            .map(|a| a.0.clone())
        );

        // Use mark-and-sweep to find other live actions.
        fn mark_recursively<'a>(
            graph: &HashMap<ActionLabel, (Box<dyn Action>, Vec<Input>)>,
            live: &mut HashSet<ActionLabel>,
            outputs: impl Iterator<Item=&'a ActionOutputLabel>,
        )
        {
            for ActionOutputLabel{action, ..} in outputs {
                if !live.insert(action.clone()) {
                    continue;
                }
                let (_, inputs) = graph.get(action)
                    .expect("Action graph is missing action");
                mark_recursively(graph, live,
                    inputs.iter().flat_map(Input::dependency));
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

        for (label, (action, inputs)) in &self.actions {
            let color = if action.is_lint() { COLOR_LINT } else { COLOR_ACTION };
            write!(f, "\"{label}\" [color = \"{color}\"];")?;
            for dependency in inputs.iter().flat_map(Input::dependency) {
                write!(f,
                    "\"{}\" -> \"{}\" [label = {}];",
                    dependency.action, label, dependency.output)?;
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
