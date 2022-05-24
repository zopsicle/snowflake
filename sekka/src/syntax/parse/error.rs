use {super::super::{lex::{self, Token}, location::Location}, thiserror::Error};

/// Result from the parser.
pub type Result<T> =
    std::result::Result<T, Error>;

/// Error returned during parsing.
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum Error
{
    #[error("{0}")]
    Lex(#[from] lex::Error),

    #[error("Expected statement, got {1}")]
    ExpectedStatement(Location, Token),

    #[error("Expected expression, got {1}")]
    ExpectedExpression(Location, Token),

    #[error("Expected identifier, got {1}")]
    ExpectedIdentifier(Location, Token),

    #[error("Expected {1}, got {2}")]
    ExpectedToken(Location, Token, Token),

    #[error("Unexpected end of file")]
    UnexpectedEof,
}
