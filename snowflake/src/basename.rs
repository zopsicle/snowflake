//! Basenames of pathnames.

use {
    std::{ffi::OsStr, os::unix::ffi::OsStrExt, path::Path, sync::Arc},
    thiserror::Error,
};

/// Basename of a pathname.
///
/// Basenames are used at various places in the build system.
/// They are used in package names, rule output names,
/// and input and output names of run command actions.
#[derive(Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct Basename
{
    // INVARIANT: See the restrictions `new` imposes.
    inner: OsStr,
}

/// Returned when a basename could not be validated.
///
/// See [`Basename::new`] for the restrictions on basenames.
#[derive(Debug, Error)]
#[error("Basename is empty, `.`, or `..`, or contains `/` or a nul")]
pub struct BasenameError;

impl Basename
{
    /// Create a basename from a string.
    ///
    /// Returns an error if the basename is invalid.
    /// A basename is invalid if it is empty, `.`, or `..`,
    /// or contains `/` or a nul.
    pub fn new<T>(inner: &T) -> Result<&Self, BasenameError>
        where T: AsRef<OsStr> + ?Sized
    {
        let inner = inner.as_ref();
        let bytes = inner.as_bytes();

        if matches!(bytes, b"" | b"." | b"..") {
            return Err(BasenameError);
        }

        if bytes.contains(&b'/') || bytes.contains(&0) {
            return Err(BasenameError);
        }

        // SAFETY: OsStr and Self have the same representation.
        Ok(unsafe { &*(inner as *const OsStr as *const Self) })
    }

    /// The underlying OS string.
    pub fn as_os_str(&self) -> &OsStr
    {
        &self.inner
    }

    /// The underlying path.
    pub fn as_path(&self) -> &Path
    {
        Path::new(self.as_os_str())
    }

    /// The underlying bytes.
    pub fn as_bytes(&self) -> &[u8]
    {
        self.as_os_str().as_bytes()
    }
}

impl From<&Basename> for Arc<Basename>
{
    fn from(other: &Basename) -> Self
    {
        let arc = Arc::<OsStr>::from(other.as_os_str());
        // SAFETY: OsStr and Basename have the same representation.
        unsafe { Arc::from_raw(Arc::into_raw(arc) as *const Basename) }
    }
}

impl AsRef<Path> for Basename
{
    fn as_ref(&self) -> &Path
    {
        self.as_path()
    }
}
