//! Icicle syntax trees and parser.

pub mod ast;
pub mod lex;
pub mod location;
pub mod parse;

#[cfg(test)]
mod tests
{
    use {
        super::{lex::{self, Lexeme, Lexer}, parse::{Arenas, parse_expression}},
        std::{ffi::OsStr, fs::{read_dir, read_to_string}, path::PathBuf},
    };

    #[test]
    fn examples()
    {
        // Locate all .icl files.
        let icl_paths: Vec<PathBuf> =
            read_dir("testdata/syntax").unwrap()
                .map(|dir_entry| dir_entry.unwrap())
                .map(|dir_entry| dir_entry.path())
                .filter(|name| name.extension() == Some(OsStr::new("icl")))
                .collect();

        for icl_path in icl_paths {

            // Read the .icl file.
            let icl = read_to_string(&icl_path).unwrap();

            // Read the corresponding .tokens file.
            let tokens_path = icl_path.with_extension("tokens");
            let tokens = read_to_string(tokens_path).unwrap();
            let tokens = tokens.trim_end();

            // Read the corresponding .ast file.
            let ast_path = icl_path.with_extension("ast");
            let ast = read_to_string(ast_path).unwrap();
            let ast = ast.trim_end();

            // Lex the contents of the .icl file.
            let actual_tokens: lex::Result<Vec<Lexeme>> =
                Lexer::new(&icl).collect();

            // Compare the tokens with the expected tokens.
            let actual_tokens = format!("{:#?}", actual_tokens);
            if actual_tokens != tokens {
                panic!(
                    "Example failed!\n\
                     Example: {icl_path:?}\n\
                     Actual tokens:\n\
                     {actual_tokens}\n\
                     Expected tokens:\n\
                     {tokens}"
                );
            }

            Arenas::with(|arenas| {

                // Parse the contents of the .icl file.
                let mut lexemes = Lexer::new(&icl).peekable();
                let actual_ast = parse_expression(arenas, &mut lexemes);

                // Compare the AST with the expected AST.
                let actual_ast = format!("{:#?}", actual_ast);
                if actual_ast != ast {
                    panic!(
                        "Example failed!\n\
                        Example: {icl_path:?}\n\
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
