//! Sekka syntax trees and parser.

pub mod ast;
pub mod lex;
pub mod location;
pub mod parse;

#[cfg(test)]
mod tests
{
    use {
        super::{lex::{self, Lexeme, Lexer}, parse::{Arenas, parse_unit}},
        std::{ffi::OsStr, fs::{read_dir, read_to_string}, path::PathBuf},
    };

    #[test]
    fn examples()
    {
        // Locate all .ska files.
        let ska_paths: Vec<PathBuf> =
            read_dir("testdata/syntax").unwrap()
                .map(|dir_entry| dir_entry.unwrap())
                .map(|dir_entry| dir_entry.path())
                .filter(|name| name.extension() == Some(OsStr::new("ska")))
                .collect();

        for ska_path in ska_paths {

            // Read the .ska file.
            let ska = read_to_string(&ska_path).unwrap();

            // Read the corresponding .tokens file.
            let tokens_path = ska_path.with_extension("tokens");
            let tokens = read_to_string(tokens_path).unwrap();
            let tokens = tokens.trim_end();

            // Read the corresponding .ast file.
            let ast_path = ska_path.with_extension("ast");
            let ast = read_to_string(ast_path).unwrap();
            let ast = ast.trim_end();

            // Lex the contents of the .ska file.
            let actual_tokens: lex::Result<Vec<Lexeme>> =
                Lexer::new(&ska).collect();

            // Compare the tokens with the expected tokens.
            let actual_tokens = format!("{:#?}", actual_tokens);
            if actual_tokens != tokens {
                panic!(
                    "Example failed!\n\
                     Example: {ska_path:?}\n\
                     Actual tokens:\n\
                     {actual_tokens}\n\
                     Expected tokens:\n\
                     {tokens}"
                );
            }

            Arenas::with(|arenas| {

                // Parse the contents of the .ska file.
                let mut lexemes = Lexer::new(&ska).peekable();
                let actual_ast = parse_unit(arenas, &mut lexemes);

                // Compare the AST with the expected AST.
                let actual_ast = format!("{:#?}", actual_ast);
                if actual_ast != ast {
                    panic!(
                        "Example failed!\n\
                        Example: {ska_path:?}\n\
                        Actual AST:\n\
                        {actual_ast}\n\
                        Expected AST:\n\
                        {ast}"
                    );
                }

            });
        }
    }
}
