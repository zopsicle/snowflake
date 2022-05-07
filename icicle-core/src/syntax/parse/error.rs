use {crate::syntax::{lex::{self, Token}, location::Location}, thiserror::Error};

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

    #[error("Expected expression, got {1}")]
    ExpectedExpression(Location, Token),

    #[error("Unexpected end of file")]
    UnexpectedEof,
}
