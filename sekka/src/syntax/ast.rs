//! Abstract syntax tree data types.

use super::location::Location;

/// Expression.
#[allow(missing_docs)]
#[derive(Debug)]
pub enum Expression<'a>
{
    /// `a + b`
    NumericAdd{
        left: &'a Expression<'a>,
        plus_sign: Location,
        right: &'a Expression<'a>,
    },

    /// `a ~ b`
    StringConcatenate{
        left: &'a Expression<'a>,
        tilde: Location,
        right: &'a Expression<'a>,
    },

    /// `'foo'`
    StringLiteral{
        location: Location,
        value: Vec<u8>,
    },
}
