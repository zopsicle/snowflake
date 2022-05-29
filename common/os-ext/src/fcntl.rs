use {
    crate::{cstr::IntoCStr, retry_on_eintr},
    std::{
        ffi::CStr,
        io,
        os::unix::io::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
    },
};

/// Equivalent to [`openat`] with [`None`] passed for `dirfd`.
pub fn open<'a>(
    pathname: impl IntoCStr<'a>,
    flags: libc::c_int,
    mode: libc::mode_t,
) -> io::Result<OwnedFd>
{
    openat(None, pathname, flags, mode)
}

/// Call openat(2) with the given arguments.
///
/// If `dirfd` is [`None`], `AT_FDCWD` is passed.
pub fn openat<'a>(
    dirfd:    Option<BorrowedFd>,
    pathname: impl IntoCStr<'a>,
    flags:    libc::c_int,
    mode:     libc::mode_t,
) -> io::Result<OwnedFd>
{
    #[inline(never)]
    fn monomorphic(
        dirfd: libc::c_int,
        pathname: &CStr,
        flags: libc::c_int,
        mode: libc::mode_t,
    ) -> io::Result<OwnedFd>
    {
        retry_on_eintr(|| {
            // SAFETY: path is NUL-terminated.
            let fd = unsafe {
                libc::openat(dirfd, pathname.as_ptr(), flags, mode)
            };

            if fd == -1 {
                return Err(io::Error::last_os_error());
            }

            // SAFETY: fd is a new, open file descriptor.
            Ok(unsafe { OwnedFd::from_raw_fd(fd) })
        })
    }

    let dirfd = dirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);
    let pathname = pathname.into_cstr()?;
    let flags = flags | libc::O_CLOEXEC;
    monomorphic(dirfd, &pathname, flags, mode)
}
