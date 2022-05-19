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

    let osx0a = ActionOutputLabel{action: asx0.clone(), output: "a".into()};
    let osx0b = ActionOutputLabel{action: asx0.clone(), output: "b".into()};
    let osx1c = ActionOutputLabel{action: asx1.clone(), output: "c".into()};
    let osy0d = ActionOutputLabel{action: asy0.clone(), output: "d".into()};

    let mut action_graph = ActionGraph{
        actions: [
            (asx0, Action::WriteRegularFile{content: "hello".into(), executable: false}),
            (asx1, Action::RunCommand{inputs: vec![osx0a, osx0b.clone()]}),
            (asy0, Action::RunCommand{inputs: vec![osx0b, osx1c.clone()]}),
            (asy1, Action::RunCommand{inputs: vec![osx1c.clone()]}),
        ].into_iter().collect(),
        artifacts: [osx1c, osy0d].into_iter().collect(),
    };

    action_graph.mark_and_sweep();

    println!("{}", action_graph);
}
