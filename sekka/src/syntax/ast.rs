//! Abstract syntax tree data types.

use {super::location::Location, std::sync::Arc};

/// Definition.
#[allow(missing_docs)]
#[derive(Debug)]
pub enum Definition<'a>
{
    /// `INIT { ... }`
    InitPhaser{
        init_keyword: Location,
        left_brace: Location,
        body: &'a [Statement<'a>],
        right_brace: Location,
    },
}

/// Statement.
#[allow(missing_docs)]
#[derive(Debug)]
pub enum Statement<'a>
{
    /// `e;`
    Expression{
        expression: Expression<'a>,
        semicolon: Location,
    },
}

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
        value: Arc<[u8]>,
    },
}
