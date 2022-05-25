use {snowflake::{action::*, basename::*, label::*}, std::sync::Arc};

fn main()
{
    let source = "INIT { } INIT { } sub f() { 'a' ~ 'b' ~ 'c'; } sub g() { }";
    let unit = sekka::unstable::compile
        ::compile_unit_from_source("".into(), source).unwrap();
    println!("{:?}", unit.init_phasers);
    println!("{:?}", unit.globals);
    let (_, procedure) = unit.constants[0].clone().to_subroutine().unwrap();
    println!("{:#?}", procedure);

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
                    outputs: [].into_iter().collect(),
                },
            ),
            (
                asy0,
                Action::RunCommand{
                    inputs: [
                        (Arc::from(Basename::new("c").unwrap()), osx01),
                        (Arc::from(Basename::new("d").unwrap()), osx10.clone()),
                    ].into_iter().collect(),
                    outputs: [].into_iter().collect(),
                },
            ),
            (
                asy1,
                Action::RunCommand{
                    inputs: [
                        (Arc::from(Basename::new("e").unwrap()), osx10.clone()),
                    ].into_iter().collect(),
                    outputs: [].into_iter().collect(),
                },
            ),
        ].into_iter().collect(),
        artifacts: [osx10, osy00].into_iter().collect(),
    };

    action_graph.prune();

    println!("{}", action_graph);
}
