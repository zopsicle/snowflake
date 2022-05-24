//! Parsing sequences of tokens into abstract syntax trees.

pub use self::{arenas::*, error::*};

use {
    self::combinators::*,
    super::{ast::*, lex::{self, Lexeme, Token}, location::Location},
    std::{iter::Peekable, sync::Arc},
};

mod arenas;
mod error;

#[macro_use]
mod combinators;

/* -------------------------------------------------------------------------- */
/*                                 Definitions                                */
/* -------------------------------------------------------------------------- */

/// Parse a unit.
pub fn parse_unit<'a>(
    arenas: &Arenas<'a>,
    lexemes: &mut Peekable<impl Iterator<Item=lex::Result<Lexeme>>>,
) -> Result<Vec<Definition<'a>>>
{
    let mut definitions = Vec::new();
    loop {
        if lexemes.peek().is_some() {
            let definition = parse_definition(arenas, lexemes)?;
            definitions.push(definition);
        } else {
            break Ok(definitions);
        }
    }
}

/// Parse a definition.
pub fn parse_definition<'a>(
    arenas: &Arenas<'a>,
    lexemes: &mut Peekable<impl Iterator<Item=lex::Result<Lexeme>>>,
) -> Result<Definition<'a>>
{
    let Lexeme{location, token} = next(lexemes)?;
    match token {
        Token::InitKeyword =>
            parse_init_phaser_definition(arenas, lexemes, location),
        Token::SubKeyword =>
            parse_subroutine_definition(arenas, lexemes, location),
        _ =>
            Err(Error::ExpectedStatement(location, token)),
    }
}

fn parse_init_phaser_definition<'a>(
    arenas: &Arenas<'a>,
    lexemes: &mut Peekable<impl Iterator<Item=lex::Result<Lexeme>>>,
    init_keyword: Location,
) -> Result<Definition<'a>>
{
    let (left_curly_bracket, body, right_curly_bracket) =
        parse_block(arenas, lexemes)?;

    let body = arenas.alloc_extend(body);
    Ok(Definition::InitPhaser{init_keyword, left_curly_bracket,
                              body, right_curly_bracket})
}

fn parse_subroutine_definition<'a>(
    arenas: &Arenas<'a>,
    lexemes: &mut Peekable<impl Iterator<Item=lex::Result<Lexeme>>>,
    sub_keyword: Location,
) -> Result<Definition<'a>>
{
    let (name_location, name) = expect_identifier(lexemes)?;
    let left_parenthesis = expect(lexemes, Token::LeftParenthesis)?;
    let right_parenthesis = expect(lexemes, Token::RightParenthesis)?;
    let (left_curly_bracket, body, right_curly_bracket) =
        parse_block(arenas, lexemes)?;

    let body = arenas.alloc_extend(body);
    Ok(Definition::Subroutine{sub_keyword, name_location, name,
                              left_parenthesis, right_parenthesis,
                              left_curly_bracket, body, right_curly_bracket})
}

/* -------------------------------------------------------------------------- */
/*                                 Statements                                 */
/* -------------------------------------------------------------------------- */

/// Parse a brace-delimited sequence of statements.
pub fn parse_block<'a>(
    arenas: &Arenas<'a>,
    lexemes: &mut Peekable<impl Iterator<Item=lex::Result<Lexeme>>>,
) -> Result<(Location, Vec<Statement<'a>>, Location)>
{
    let left_curly_bracket = expect(lexemes, Token::LeftCurlyBracket)?;
    let (body, right_curly_bracket) =
        many_until(lexemes,
            |lexemes| parse_statement(arenas, lexemes),
            Token::RightCurlyBracket,
        )?;
    Ok((left_curly_bracket, body, right_curly_bracket))
}

/// Parse a statement.
pub fn parse_statement<'a>(
    arenas: &Arenas<'a>,
    lexemes: &mut Peekable<impl Iterator<Item=lex::Result<Lexeme>>>,
) -> Result<Statement<'a>>
{
    let expression = parse_expression(arenas, lexemes)?;
    let semicolon = expect(lexemes, Token::Semicolon)?;
    Ok(Statement::Expression{expression, semicolon})
}

/* -------------------------------------------------------------------------- */
/*                                 Expressions                                */
/* -------------------------------------------------------------------------- */

/// Parse an expression.
pub fn parse_expression<'a>(
    arenas: &Arenas<'a>,
    lexemes: &mut Peekable<impl Iterator<Item=lex::Result<Lexeme>>>,
) -> Result<Expression<'a>>
{
    parse_expression_2(arenas, lexemes)
}

fn parse_expression_2<'a>(
    arenas: &Arenas<'a>,
    lexemes: &mut Peekable<impl Iterator<Item=lex::Result<Lexeme>>>,
) -> Result<Expression<'a>>
{
    left_associative!(arenas, lexemes, parse_expression_1, match {
        Token::PlusSign => |left, plus_sign, right|
            Expression::NumericAdd{left, plus_sign, right},
        Token::Tilde => |left, tilde, right|
            Expression::StringConcatenate{left, tilde, right},
    })
}

fn parse_expression_1(lexemes: &mut impl Iterator<Item=lex::Result<Lexeme>>)
    -> Result<Expression<'static>>
{
    let Lexeme{location, token} = next(lexemes)?;
    match token {
        Token::StringLiteral(value) =>
            Ok(Expression::StringLiteral{location, value}),
        _ =>
            Err(Error::ExpectedExpression(location, token)),
    }
}

/* -------------------------------------------------------------------------- */
/*                                  Terminals                                 */
/* -------------------------------------------------------------------------- */

fn expect_identifier(
    lexemes: &mut impl Iterator<Item=lex::Result<Lexeme>>,
) -> Result<(Location, Arc<str>)>
{
    expect_match!(lexemes,
        Token::Identifier(name) => name,
        _ => Error::ExpectedIdentifier,
    )
}
