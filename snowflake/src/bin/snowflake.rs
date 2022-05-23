use {
    sekka::{
        Isolate,
        ir::{Builder, lower_expression},
        syntax::{
            lex::Lexer,
            location::Location,
            parse::{Arenas, parse_expression},
        },
    },
    snowflake::{action::*, basename::*, label::*},
    std::sync::Arc,
};

fn main()
{
    {
        Arenas::with(|arenas| {

            let lexer = Lexer::new("'a' ~ 'bc' ~ 'def'");
            let ast = parse_expression(arenas, &mut lexer.peekable()).unwrap();

            let mut b = Builder::new();
            let result = lower_expression(&mut b, &ast);
            b.set_location(Location{offset: 0});
            b.build_return(result);
            let instructions = b.finish();

            let sekka = Isolate::new();
            sekka.run(&instructions);

        });
    }

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
