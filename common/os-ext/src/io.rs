//! Working with file descriptors.

use std::{
    io,
    mem::ManuallyDrop,
    os::unix::io::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
};

/// Extra methods for [`BorrowedFd`].
pub trait BorrowedFdExt: Sized
{
    /// Analogous to [`OwnedFd::try_clone`].
    fn try_to_owned(self) -> io::Result<OwnedFd>;
}

impl BorrowedFdExt for BorrowedFd<'_>
{
    fn try_to_owned(self) -> io::Result<OwnedFd>
    {
        // SAFETY: We only use it to call try_clone and don't drop it.
        let owned = unsafe { OwnedFd::from_raw_fd(self.as_raw_fd()) };
        ManuallyDrop::new(owned).try_clone()
    }
}
