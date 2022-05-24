use {super::super::location::Location, std::{fmt, sync::Arc}};

/// Token along with its location.
#[allow(missing_docs)]
#[derive(Debug)]
pub struct Lexeme
{
    pub location: Location,
    pub token: Token,
}

/// Structured information about a lexeme.
#[derive(Debug, Eq, PartialEq)]
pub enum Token
{
    /** `(` */ LeftParenthesis,
    /** `)` */ RightParenthesis,
    /** `+` */ PlusSign,
    /** `;` */ Semicolon,
    /** `{` */ LeftCurlyBracket,
    /** `}` */ RightCurlyBracket,
    /** `~` */ Tilde,

    /** `INIT` */ InitKeyword,
    /** `sub`  */ SubKeyword,

    /// Identifier.
    Identifier(Arc<str>),

    /// String literal.
    ///
    /// The contained string is the actual string value;
    /// any escape sequences have already been resolved.
    StringLiteral(Arc<[u8]>),
}

impl fmt::Display for Token
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        match self {
            Self::LeftParenthesis   => write!(f, "`(`"),
            Self::RightParenthesis  => write!(f, "`)`"),
            Self::PlusSign          => write!(f, "`+`"),
            Self::Semicolon         => write!(f, "`;`"),
            Self::LeftCurlyBracket  => write!(f, "`{{`"),
            Self::RightCurlyBracket => write!(f, "`}}`"),
            Self::Tilde             => write!(f, "`~`"),
            Self::InitKeyword       => write!(f, "`INIT`"),
            Self::SubKeyword        => write!(f, "`sub`"),
            Self::Identifier(..)    => write!(f, "identifier"),
            Self::StringLiteral(..) => write!(f, "string literal"),
        }
    }
}
