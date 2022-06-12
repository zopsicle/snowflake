//! Identifying elements of a cache.

pub use self::{blake3::*, file::*};

use {serde::{Deserialize, Serialize}, std::{fmt, str::from_utf8_unchecked}};

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
#[derive(Clone, Copy, Deserialize, Eq, PartialEq, Serialize)]
pub struct Hash(pub [u8; 32]);

impl fmt::Display for Hash
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        const ALPHABET: &[u8; 16] = b"0123456789abcdef";
        let mut buf = [0; 64];
        for (i, &b) in self.0.iter().enumerate() {
            buf[2 * i + 0] = ALPHABET[b as usize >> 4];
            buf[2 * i + 1] = ALPHABET[b as usize & 0b1111];
        }
        // SAFETY: We filled the buffer with ASCII characters.
        let str = unsafe { from_utf8_unchecked(&buf) };
        write!(f, "{}", str)
    }
}

impl fmt::Debug for Hash
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        write!(f, "\"{self}\"")
    }
}
