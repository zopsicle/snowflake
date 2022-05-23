//! Parsing sequences of tokens into abstract syntax trees.

pub use self::{arenas::*, error::*};

use {
    self::combinators::*,
    super::{ast::*, lex::{self, Lexeme, Token}, location::Location},
    std::iter::Peekable,
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
        _ => Err(Error::ExpectedStatement(location, token)),
    }
}

fn parse_init_phaser_definition<'a>(
    arenas: &Arenas<'a>,
    lexemes: &mut Peekable<impl Iterator<Item=lex::Result<Lexeme>>>,
    init_keyword: Location,
) -> Result<Definition<'a>>
{
    let left_brace = expect(lexemes, Token::LeftCurlyBracket)?;
    let (body, right_brace) =
        many_until(lexemes,
            |lexemes| parse_statement(arenas, lexemes),
            Token::RightCurlyBracket,
        )?;
    let body = arenas.alloc_extend(body);
    Ok(Definition::InitPhaser{init_keyword, left_brace, body, right_brace})
}

/* -------------------------------------------------------------------------- */
/*                                 Statements                                 */
/* -------------------------------------------------------------------------- */

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
