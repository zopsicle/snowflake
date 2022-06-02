//! Identifying elements of a cache.

pub use self::{blake3::*, file::*};

use std::fmt;

mod blake3;
mod file;
mod put;

/// Cryptographic hash used for identifying elements of a cache.
///
/// # Examples
///
/// A hash can be computed using [`Blake3`].
/// Displaying a hash produces a lower-case hexadecimal string.
///
/// ```
/// use snowflake_util::hash::Blake3;
/// let hash = Blake3::new().update(b"Hello, world!").finalize();
/// assert_eq!(hash.to_string(), "ede5c0b10f2ec4979c69b52f61e42ff5\
///                               b413519ce09be0f14d098dcfe5f6f98d");
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Hash(pub [u8; 32]);

impl fmt::Display for Hash
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        for byte in self.0 {
            write!(f, "{:02x}", byte)?;
        }
        Ok(())
    }
}
