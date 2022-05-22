use thiserror::Error;

/// Result from the lexer.
pub type Result<T> =
    std::result::Result<T, Error>;

/// Error returned during lexing.
#[allow(missing_docs)]
#[derive(Clone, Debug, Error)]
pub enum Error
{
    #[error("Invalid token: {0:?}")]
    InvalidToken(char),

    #[error("Unterminated string literal")]
    UnterminatedStringLiteral,
}
