use {
    regex::bytes::Regex,
    sekka::Sekka,
    snowflake::{action::*, basename::*, label::*},
    std::{time::Duration, sync::Arc},
};

fn main()
{
    let source = "INIT { } INIT { } sub f() { 'a' ~ 'b' ~ 'c'; } sub g() { }";
    let mut sekka = Sekka::new();
    sekka.compile("".into(), source).unwrap();

    let ps = PackageLabel{segments: vec![].into()};

    let rsx = RuleLabel{package: ps.clone(), rule: "x".into()};
    let rsy = RuleLabel{package: ps.clone(), rule: "y".into()};
    let rsz = RuleLabel{package: ps,         rule: "z".into()};

    let asx0 = ActionLabel{rule: rsx.clone(), action: 0};
    let asx1 = ActionLabel{rule: rsx,         action: 1};
    let asy0 = ActionLabel{rule: rsy.clone(), action: 0};
    let asy1 = ActionLabel{rule: rsy,         action: 1};
    let asz0 = ActionLabel{rule: rsz,         action: 0};

    let osx00 = ActionOutputLabel{action: asx0.clone(), output: 0};
    let osx01 = ActionOutputLabel{action: asx0.clone(), output: 1};
    let osx10 = ActionOutputLabel{action: asx1.clone(), output: 0};
    let osy00 = ActionOutputLabel{action: asy0.clone(), output: 0};

    let mut action_graph = ActionGraph{
        actions: [
            (
                asx0,
                Action::WriteRegularFile{
                    content: "hello".into(),
                    executable: false,
                },
            ),
            (
                asx1,
                Action::RunCommand{
                    inputs: [
                        (Arc::from(Basename::new("a").unwrap()), osx00),
                        (Arc::from(Basename::new("b").unwrap()), osx01.clone()),
                    ].into_iter().collect(),
                    outputs: vec![Arc::from(Basename::new("o").unwrap())],
                    program: "/run/current-system/sw/bin/sleep".into(),
                    arguments: vec![],
                    environment: vec![],
                    timeout: Duration::from_secs(60),
                    warnings: None,
                },
            ),
            (
                asy0,
                Action::RunCommand{
                    inputs: [
                        (Arc::from(Basename::new("c").unwrap()), osx01),
                        (Arc::from(Basename::new("d").unwrap()), osx10.clone()),
                    ].into_iter().collect(),
                    outputs: vec![Arc::from(Basename::new("o").unwrap())],
                    program: "/run/current-system/sw/bin/sleep".into(),
                    arguments: vec![],
                    environment: vec![],
                    timeout: Duration::from_secs(60),
                    warnings: None,
                },
            ),
            (
                asy1,
                Action::RunCommand{
                    inputs: [
                        (Arc::from(Basename::new("e").unwrap()), osx10.clone()),
                    ].into_iter().collect(),
                    outputs: vec![Arc::from(Basename::new("o").unwrap())],
                    program: "/run/current-system/sw/bin/sleep".into(),
                    arguments: vec![],
                    environment: vec![],
                    timeout: Duration::from_secs(60),
                    warnings: Some(Regex::new("^warning:").unwrap()),
                },
            ),
            (
                asz0,
                Action::RunCommand{
                    inputs: [
                        (Arc::from(Basename::new("f").unwrap()), osx10.clone()),
                    ].into_iter().collect(),
                    outputs: [].into_iter().collect(),
                    program: "/run/current-system/sw/bin/sleep".into(),
                    arguments: vec![],
                    environment: vec![],
                    timeout: Duration::from_secs(60),
                    warnings: Some(Regex::new("").unwrap()),
                },
            ),
        ].into_iter().collect(),
        artifacts: [osx10, osy00].into_iter().collect(),
    };

    action_graph.prune();

    println!("{}", action_graph);
}
