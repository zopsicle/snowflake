//! Basenames of pathnames.

// TODO: Move this to its own crate.

use {
    std::{ffi::OsStr, fmt, ops::Deref, os::unix::ffi::OsStrExt},
    thiserror::Error,
};

/// Basename of a pathname.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Basename<T>
    where T: ?Sized
{
    // INVARIANT: See the restrictions `new` imposes.
    inner: T,
}

/// Returned when a basename could not be validated.
///
/// See [`Basename::new`] for the restrictions on basenames.
#[derive(Debug, Error)]
#[error("Basename is empty, `.`, or `..`, or contains `/` or a nul")]
pub struct BasenameError;

impl<T> Basename<T>
    where T: AsRef<OsStr>
{
    /// Create a basename from a string.
    ///
    /// Returns an error if the basename is invalid.
    /// A basename is invalid if it is empty, `.`, or `..`,
    /// or contains `/` or a nul.
    pub fn new(inner: T) -> Result<Self, BasenameError>
    {
        let bytes = inner.as_ref().as_bytes();

        if matches!(bytes, b"" | b"." | b"..") {
            return Err(BasenameError);
        }

        if bytes.contains(&b'/') || bytes.contains(&0) {
            return Err(BasenameError);
        }

        Ok(Self{inner})
    }
}

impl<T> Deref for Basename<T>
    where T: ?Sized
{
    type Target = T;

    fn deref(&self) -> &Self::Target
    {
        &self.inner
    }
}

impl<T> fmt::Debug for Basename<T>
    where T: fmt::Debug + ?Sized
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        <T as fmt::Debug>::fmt(self, f)
    }
}
