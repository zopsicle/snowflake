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
