#![feature(concat_bytes)]
#![feature(io_safety)]
#![feature(let_chains)]

use {
    os_ext::{O_DIRECTORY, O_PATH, cstr, cstring, mkdir, open},
    regex::bytes::Regex,
    snowflake_actions::*,
    snowflake_core::{action::*, drive::{self, drive}, label::*, state::State},
    snowflake_util::basename::*,
    std::{
        ffi::CString,
        io::ErrorKind::AlreadyExists,
        os::unix::io::AsFd,
        time::Duration,
    },
};

fn main()
{
    let minify = CString::new(concat!(env!("SNOWFLAKE_MINIFY"), "/bin/minify")).unwrap();
    let sassc = CString::new(concat!(env!("SNOWFLAKE_SASSC"), "/bin/sassc")).unwrap();

    let action_sassc = ActionLabel{action: 0};
    let action_sassc_output_css = ActionOutputLabel{action: action_sassc.clone(), output: 0};

    let action_minify = ActionLabel{action: 1};
    let action_minify_output_min_css = ActionOutputLabel{action: action_minify.clone(), output: 0};

    let mut action_graph = ActionGraph{
        actions: [
            (
                action_sassc,
                (
                    Box::new(RunCommand{
                        inputs: vec![
                            Basename::new(cstring!(b"stylesheet.scss")).unwrap(),
                        ],
                        outputs: Outputs::Outputs(vec![
                            Basename::new(cstring!(b"stylesheet.css")).unwrap(),
                        ]),
                        program: sassc,
                        arguments: vec![
                            cstring!(b"sassc"),
                            cstring!(b"stylesheet.scss"),
                            cstring!(b"stylesheet.css"),
                        ],
                        environment: vec![],
                        timeout: Duration::from_secs(1),
                        warnings: Some(Regex::new("^WARNING:").unwrap()),
                    }) as Box<dyn Action>,
                    vec![
                        Input::StaticFile(cstring!(b"snowflake-website/stylesheet.scss")),
                    ],
                ),
            ),
            (
                action_minify,
                (
                    Box::new(RunCommand{
                        inputs: vec![
                            Basename::new(cstring!(b"stylesheet.css")).unwrap(),
                        ],
                        outputs: Outputs::Outputs(vec![
                            Basename::new(cstring!(b"stylesheet.min.css")).unwrap(),
                        ]),
                        program: minify,
                        arguments: vec![
                            cstring!(b"minify"),
                            cstring!(b"--output"),
                            cstring!(b"stylesheet.min.css"),
                            cstring!(b"stylesheet.css"),
                        ],
                        environment: vec![],
                        timeout: Duration::from_secs(1),
                        warnings: None,
                    }) as Box<dyn Action>,
                    vec![
                        Input::Dependency(action_sassc_output_css),
                    ],
                ),
            ),
        ].into_iter().collect(),
        artifacts: [action_minify_output_min_css].into_iter().collect(),
    };

    action_graph.prune();

    if let Err(err) = mkdir(cstr!(b".snowflake"), 0o755)
        && err.kind() != AlreadyExists {
        panic!("{:?}", err);
    }
    let state = State::open(cstr!(b".snowflake")).unwrap();
    let source_root = open(cstr!(b"."), O_DIRECTORY | O_PATH, 0).unwrap();
    let context = drive::Context{state: &state, source_root: source_root.as_fd()};
    let result = drive(&context, &action_graph);

    println!("{}", action_graph);
    println!("{:?}", result);
}
