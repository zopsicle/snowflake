use {
    crate::syntax::{lex::{self, Lexeme, Token}, location::Location},
    super::{Error, Result},
    std::iter::Peekable,
};

/// Read a lexeme.
pub fn next(lexemes: &mut impl Iterator<Item=lex::Result<Lexeme>>)
    -> Result<Lexeme>
{
    match lexemes.next() {
        Some(Ok(lexeme)) => Ok(lexeme),
        Some(Err(err))   => Err(err.into()),
        None             => Err(Error::UnexpectedEof),
    }
}

/// Read a lexeme but do not consume it.
pub fn peek(lexemes: &mut Peekable<impl Iterator<Item=lex::Result<Lexeme>>>)
    -> Result<Option<&Lexeme>>
{
    match lexemes.peek() {
        None             => Ok(None),
        Some(Err(err))   => Err(err.clone().into()),
        Some(Ok(lexeme)) => Ok(Some(lexeme)),
    }
}

/// Read a lexeme if it matches a predicate.
pub fn next_if(
    lexemes: &mut Peekable<impl Iterator<Item=lex::Result<Lexeme>>>,
    f: impl FnOnce(&Token) -> bool,
) -> Result<Option<Location>>
{
    next_if_with(lexemes, |lexeme| f(&lexeme.token).then(|| lexeme.location))
}

/// Read a lexeme if it matches a predicate.
///
/// The value returned by the predicate is returned from the combinator.
/// `f` may mutate the lexeme, but only if it subsequently returns [`Some`].
pub fn next_if_with<R>(
    lexemes: &mut Peekable<impl Iterator<Item=lex::Result<Lexeme>>>,
    f: impl FnOnce(&mut Lexeme) -> Option<R>,
) -> Result<Option<R>>
{
    match lexemes.peek_mut() {
        None             => Ok(None),
        Some(Err(err))   => Err(err.clone().into()),
        Some(Ok(lexeme)) =>
            if let Some(r) = f(lexeme) {
                lexemes.next();
                Ok(Some(r))
            } else {
                Ok(None)
            },
    }
}

/// Read a lexeme if it matches a pattern.
macro_rules! next_if_matches
{
    ($lexemes:expr, $pattern:pat) => {
        next_if($lexemes, |token| matches!(token, $pattern))
    };
}

/// Parse left-associative binary operators at the same precedence.
macro_rules! left_associative
{
    (
        $arenas:expr,
        $lexemes:expr,
        $parse_next_precedence:ident,
        match {
            $($oppat:pat => |$left:ident, $op:ident, $right:ident| $then:expr),*
            $(,)?
        }
    ) => {{
        let arenas = $arenas;
        let lexemes = $lexemes;
        let mut left = $parse_next_precedence(lexemes)?;
        loop {
            $(
                if let Some($op) = next_if_matches!(lexemes, $oppat)? {
                    let right = $parse_next_precedence(lexemes)?;
                    let $left  = arenas.alloc(left);
                    let $right = arenas.alloc(right);
                    left = $then;
                    continue;
                }
            )*
            break;
        }
        Ok(left)
    }};
}
