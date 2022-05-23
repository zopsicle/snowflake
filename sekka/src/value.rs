use {std::sync::Arc, thiserror::Error};

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

#[derive(Debug, Error)]
#[error("String value would be too large")]
pub struct StringFromBytesError;

#[derive(Debug, Error)]
pub enum ToStringError
{
    #[error("Use of undef in string context")]
    Undef,
}
