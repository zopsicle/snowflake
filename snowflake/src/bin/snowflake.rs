#![feature(concat_bytes)]

use {
    os_ext::cstring,
    regex::bytes::Regex,
    snowflake_actions::*,
    snowflake_core::{action::*, label::*},
    snowflake_util::basename::*,
    std::time::Duration,
};

fn main()
{
    let asx0 = ActionLabel{action: 0};
    let asx1 = ActionLabel{action: 1};
    let asy0 = ActionLabel{action: 0};
    let asy1 = ActionLabel{action: 1};
    let asz0 = ActionLabel{action: 0};

    let osx00 = ActionOutputLabel{action: asx0.clone(), output: 0};
    let osx01 = ActionOutputLabel{action: asx0.clone(), output: 1};
    let osx10 = ActionOutputLabel{action: asx1.clone(), output: 0};
    let osy00 = ActionOutputLabel{action: asy0.clone(), output: 0};

    let mut action_graph = ActionGraph{
        actions: [
            (
                asx0,
                (
                    Box::new(WriteRegularFile{
                        content: "hello".into(),
                        executable: false,
                    }) as Box<dyn Action>,
                    vec![],
                ),
            ),
            (
                asx1,
                (
                    Box::new(RunCommand{
                        inputs: vec![
                            Basename::new(cstring!(b"a")).unwrap(),
                            Basename::new(cstring!(b"b")).unwrap(),
                        ],
                        outputs: Outputs::Outputs(
                            vec![Basename::new(cstring!(b"o")).unwrap()],
                        ),
                        program: cstring!(b"/run/current-system/sw/bin/sleep"),
                        arguments: vec![],
                        environment: vec![],
                        timeout: Duration::from_secs(60),
                        warnings: None,
                    }) as Box<dyn Action>,
                    vec![
                        Input::Dependency(osx00),
                        Input::Dependency(osx01.clone()),
                    ],
                ),
            ),
            (
                asy0,
                (
                    Box::new(RunCommand{
                        inputs: vec![
                            Basename::new(cstring!(b"c")).unwrap(),
                            Basename::new(cstring!(b"d")).unwrap(),
                        ],
                        outputs: Outputs::Outputs(
                            vec![Basename::new(cstring!(b"o")).unwrap()],
                        ),
                        program: cstring!(b"/run/current-system/sw/bin/sleep"),
                        arguments: vec![],
                        environment: vec![],
                        timeout: Duration::from_secs(60),
                        warnings: None,
                    }) as Box<dyn Action>,
                    vec![
                        Input::Dependency(osx01),
                        Input::Dependency(osx10.clone()),
                    ],
                ),
            ),
            (
                asy1,
                (
                    Box::new(RunCommand{
                        inputs: vec![
                            Basename::new(cstring!(b"e")).unwrap(),
                        ],
                        outputs: Outputs::Outputs(
                            vec![Basename::new(cstring!(b"o")).unwrap()],
                        ),
                        program: cstring!(b"/run/current-system/sw/bin/sleep"),
                        arguments: vec![],
                        environment: vec![],
                        timeout: Duration::from_secs(60),
                        warnings: Some(Regex::new("^warning:").unwrap()),
                    }) as Box<dyn Action>,
                    vec![
                        Input::Dependency(osx10.clone()),
                    ],
                ),
            ),
            (
                asz0,
                (
                    Box::new(RunCommand{
                        inputs: vec![
                            Basename::new(cstring!(b"f")).unwrap(),
                        ],
                        outputs: Outputs::Outputs(
                            [].into_iter().collect(),
                        ),
                        program: cstring!(b"/run/current-system/sw/bin/sleep"),
                        arguments: vec![],
                        environment: vec![],
                        timeout: Duration::from_secs(60),
                        warnings: Some(Regex::new("").unwrap()),
                    }) as Box<dyn Action>,
                    vec![
                        Input::Dependency(osx10.clone()),
                    ],
                ),
            ),
        ].into_iter().collect(),
        artifacts: [osx10, osy00].into_iter().collect(),
    };

    action_graph.prune();

    println!("{}", action_graph);
}
