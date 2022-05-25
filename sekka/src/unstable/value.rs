//! Reference-counted Sekka values.

use {
    crate::unstable::bytecode::Verified,
    std::{error::Error, sync::Arc},
    thiserror::Error,
};

/// Reference-counted Sekka value.
#[derive(Clone)]
pub struct Value
{
    inner: Inner,
}

#[derive(Clone)]
enum Inner
{
    // The current implementation is a very very fat pointer.
    // A future version should optimize this using unions.
    // To remain future-proof, constructors should limit
    // any length fields to u32::MAX using assertions.

    Undef,
    Boolean(bool),
    String(Arc<[u8]>),
    Error(Arc<dyn 'static + Error + Send + Sync>),
    Subroutine(Arc<[Value]>, Arc<Verified>),
}

impl Value
{
    /// Create the undef value.
    pub fn undef() -> Self
    {
        Self{inner: Inner::Undef}
    }

    /// Create a Boolean value.
    pub fn boolean_from_bool(value: bool) -> Self
    {
        Self{inner: Inner::Boolean(value)}
    }

    /// Create a string value from the bytes that make it up.
    pub fn string_from_bytes(bytes: Arc<[u8]>)
        -> Result<Self, StringFromBytesError>
    {
        if bytes.len() > u32::MAX as usize {
            return Err(StringFromBytesError);
        }
        Ok(Self{inner: Inner::String(bytes)})
    }

    /// Create an error value from an error.
    pub fn error_from_error<E>(error: E) -> Self
        where E: 'static + Error + Send + Sync
    {
        Self{inner: Inner::Error(Arc::new(error))}
    }

    /// Create a subroutine value from a procedure.
    ///
    /// The environment of the subroutine is left empty,
    /// so this shall only be used for non-closures.
    pub fn subroutine_from_procedure(procedure: Arc<Verified>) -> Self
    {
        let environment = Arc::from(&[][..]);
        Self::subroutine_from_environment_and_procedure(environment, procedure)
    }

    /// Create a subroutine value from an environment and a procedure.
    pub fn subroutine_from_environment_and_procedure(
        environment: Arc<[Value]>,
        procedure: Arc<Verified>,
    ) -> Self
    {
        Self{inner: Inner::Subroutine(environment, procedure)}
    }

    /// Convert the value to a string.
    pub fn to_string(self) -> Result<Arc<[u8]>, ToStringError>
    {
        match self.inner {
            Inner::Undef =>
                Err(ToStringError::Undef),
            Inner::Boolean(value) =>
                match value {
                    true  => Ok(b"true"[..].into()),
                    false => Ok(b"false"[..].into()),
                },
            Inner::String(bytes) =>
                Ok(bytes),
            Inner::Error(error) =>
                Ok(error.to_string().into_bytes().into()),
            Inner::Subroutine{..} =>
                Err(ToStringError::Subroutine),
        }
    }

    /// Convert the value to a subroutine.
    pub fn to_subroutine(self) -> Result<(Arc<[Value]>, Arc<Verified>), ()>
    {
        match self.inner {
            Inner::Subroutine(e, p) => Ok((e, p)),
            _ => Err(()),
        }
    }
}

/// Create a string value from format arguments.
///
/// # Panics
///
/// Panics if the resulting string would have a length
/// that exceeds the maximum length for string values.
#[macro_export]
macro_rules! string_from_format
{
    ($($arg:tt)*) => {
        $crate::value::Value::string_from_bytes(
            ::std::format!($($arg)*).into_bytes().into()
        ).unwrap()
    };
}

/// Error returned by [`Value::string_from_bytes`].
#[derive(Debug, Error)]
#[error("String value would be too large")]
pub struct StringFromBytesError;

/// Error returned by [`Value::to_string`].
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum ToStringError
{
    #[error("Use of undef in string context")]
    Undef,

    #[error("Use of subroutine in string context")]
    Subroutine,
}
