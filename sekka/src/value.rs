//! Working with Sekka values.

// NOTE: The current implementation is very inefficient.
// A future version should use more efficient value representations.
// And a far future version should implement a garbage collector.

use {crate::bytecode::Verified, num_bigint::BigInt, std::sync::{Arc, Mutex}};

/// Reference counted Sekka value.
#[derive(Clone)]
pub struct Value
{
    inner: Arc<Inner>,
}

enum Inner
{
    Undef,
    Boolean(bool),
    Integer(BigInt),
    String(Vec<u8>),
    Subroutine{
        environment: Vec<Value>,
        procedure: Arc<Verified>,
    },
    Slot(Mutex<Value>),
}

/// Borrowed view into a value.
#[allow(missing_docs)]
pub enum View<'a>
{
    Undef,
    Boolean(bool),
    Integer(&'a BigInt),
    String(&'a [u8]),
    Subroutine{
        environment: &'a [Value],
        procedure: &'a Arc<Verified>,
    },
    Slot(&'a Mutex<Value>),
}

impl Value
{
    /// Borrow the value.
    pub fn view(&self) -> View
    {
        match &*self.inner {
            Inner::Undef => View::Undef,
            Inner::Boolean(value) => View::Boolean(*value),
            Inner::Integer(value) => View::Integer(value),
            Inner::String(value) => View::String(value),
            Inner::Subroutine{environment, procedure} =>
                View::Subroutine{environment, procedure},
            Inner::Slot(mutex) => View::Slot(mutex),
        }
    }

    /// Obtain the undef value.
    pub fn undef() -> Self
    {
        let inner = Inner::Undef;
        Self{inner: Arc::new(inner)}
    }

    /// Obtain the Boolean value for a given Boolean.
    pub fn boolean_from_bool(value: bool) -> Self
    {
        let inner = Inner::Boolean(value);
        Self{inner: Arc::new(inner)}
    }

    /// Obtain the integer value for a given integer.
    pub fn integer_from_i64(value: i64) -> Self
    {
        let inner = Inner::Integer(value.into());
        Self{inner: Arc::new(inner)}
    }

    /// Obtain the string value for a given string.
    pub fn string_from_bytes(bytes: &[u8]) -> Self
    {
        let inner = Inner::String(bytes.into());
        Self{inner: Arc::new(inner)}
    }

    /// Obtain the subroutine value for a given
    /// environment and procedure.
    pub fn subroutine_from_environment_and_procedure(
        environment: &[Value],
        procedure: Arc<Verified>,
    ) -> Self
    {
        let environment = environment.into();
        let inner = Inner::Subroutine{environment, procedure};
        Self{inner: Arc::new(inner)}
    }

    /// Create a slot value from a value.
    pub fn slot_from_value(value: Value) -> Self
    {
        let inner = Inner::Slot(Mutex::new(value));
        Self{inner: Arc::new(inner)}
    }
}
