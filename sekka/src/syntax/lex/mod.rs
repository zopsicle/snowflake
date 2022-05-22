//! Splitting source code into sequences of tokens.

pub use self::{error::*, lexeme::*};

use {super::location::Location, std::{iter::Peekable, str::CharIndices}};

mod error;
mod lexeme;

/// Splits source code into a sequence of tokens.
pub struct Lexer<'a>
{
    chars: Peekable<CharIndices<'a>>,
}

impl<'a> Lexer<'a>
{
    /// Create a lexer for some source code.
    pub fn new(input: &'a str) -> Self
    {
        Self{chars: input.char_indices().peekable()}
    }

    fn read_lexeme(&mut self) -> Option<Result<Lexeme>>
    {
        self.read_whitespace();
        if let Some((i, c)) = self.chars.next() {
            let location = Location{offset: i};
            Some(self.read_token(c).map(|token| Lexeme{location, token}))
        } else {
            None
        }
    }

    fn read_whitespace(&mut self)
    {
        while self.chars.next_if(|&(_, c)| Self::is_whitespace(c)).is_some() {
        }
    }

    fn read_token(&mut self, c: char) -> Result<Token>
    {
        match c {
            '~'  => Ok(Token::Tilde),
            '\'' => self.read_single_quoted_string_literal(),
            _    => Err(Error::InvalidToken(c)),
        }
    }

    fn read_single_quoted_string_literal(&mut self) -> Result<Token>
    {
        let mut string = Vec::new();
        loop {
            match self.chars.next() {
                None            => break Err(Error::UnterminatedStringLiteral),
                Some((_, '\'')) => break Ok(Token::StringLiteral(string)),
                Some((_, c))    => {
                    let mut utf8 = [0; 4];
                    let utf8 = c.encode_utf8(&mut utf8);
                    string.extend_from_slice(utf8.as_bytes());
                },
            }
        }
    }

    fn is_whitespace(c: char) -> bool
    {
        matches!(c, ' ' | '\t' | '\r' | '\n')
    }
}

impl<'a> Iterator for Lexer<'a>
{
    type Item = Result<Lexeme>;

    fn next(&mut self) -> Option<Self::Item>
    {
        self.read_lexeme()
    }
}
