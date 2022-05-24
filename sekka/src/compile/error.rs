use {
    crate::{bytecode::BuildError, syntax::parse, value::StringFromBytesError},
    std::sync::Arc,
    thiserror::Error,
};

/// Compilation result.
pub type Result<T> =
    std::result::Result<T, Error>;

/// Compilation error.
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum Error
{
    #[error("{0}")]
    Parse(#[from] parse::Error),

    #[error("Too many constants")]
    TooManyConstants,

    #[error("Redefinition of `{0}`")]
    Redefinition(Arc<str>),

    #[error("{0}")]
    Build(#[from] BuildError),

    #[error("{0}")]
    StringFromBytes(#[from] StringFromBytesError),
}
