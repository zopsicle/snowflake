use {super::super::location::Location, std::fmt};

/// Token along with its location.
#[allow(missing_docs)]
#[derive(Debug)]
pub struct Lexeme
{
    pub location: Location,
    pub token: Token,
}

/// Structured information about a lexeme.
#[derive(Debug)]
pub enum Token
{
    /** `+` */ PlusSign,
    /** `~` */ Tilde,

    /// String literal.
    ///
    /// The contained string is the actual string value;
    /// any escape sequences have already been resolved.
    StringLiteral(Vec<u8>),
}

impl fmt::Display for Token
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        match self {
            Self::PlusSign          => write!(f, "`+`"),
            Self::Tilde             => write!(f, "`~`"),
            Self::StringLiteral(..) => write!(f, "string literal"),
        }
    }
}
