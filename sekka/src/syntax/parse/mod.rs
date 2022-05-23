//! Parsing sequences of tokens into abstract syntax trees.

pub use self::{arenas::*, error::*};

use {
    self::combinators::*,
    super::{ast::*, lex::{self, Lexeme, Token}},
    std::iter::Peekable,
};

mod arenas;
mod error;

#[macro_use]
mod combinators;

/// Parse a unit.
pub fn parse_unit<'a>(
    arenas: &Arenas<'a>,
    lexemes: &mut Peekable<impl Iterator<Item=lex::Result<Lexeme>>>,
) -> Result<Unit<'a>>
{
    // TODO: Parse many statements.
    let expression = parse_expression(arenas, lexemes)?;
    let semicolon = next(lexemes)?;

    assert!(matches!(semicolon.token, Token::Semicolon));
    let semicolon = semicolon.location;

    let statement = Statement::Expression{expression, semicolon};
    Ok(Unit{statements: vec![statement]})
}

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
