use snowflake::{action::*, label::*};

fn main()
{
    let ps = PackageLabel{segments: vec![].into()};

    let rsx = RuleLabel{package: ps.clone(), rule: "x".into()};
    let rsy = RuleLabel{package: ps,         rule: "y".into()};

    let asx0 = ActionLabel{rule: rsx.clone(), action: 0};
    let asx1 = ActionLabel{rule: rsx,         action: 1};
    let asy0 = ActionLabel{rule: rsy.clone(), action: 0};
    let asy1 = ActionLabel{rule: rsy,         action: 1};

    let osx00 = ActionOutputLabel{action: asx0.clone(), output: 0};
    let osx01 = ActionOutputLabel{action: asx0.clone(), output: 1};
    let osx10 = ActionOutputLabel{action: asx1.clone(), output: 0};
    let osy00 = ActionOutputLabel{action: asy0.clone(), output: 0};

    let mut action_graph = ActionGraph{
        actions: [
            (asx0, Action::WriteRegularFile{content: "hello".into(), executable: false}),
            (asx1, Action::RunCommand{inputs: vec![osx00, osx01.clone()]}),
            (asy0, Action::RunCommand{inputs: vec![osx01, osx10.clone()]}),
            (asy1, Action::RunCommand{inputs: vec![osx10.clone()]}),
        ].into_iter().collect(),
        artifacts: [osx10, osy00].into_iter().collect(),
    };

    action_graph.prune();

    println!("{}", action_graph);
}
