use std::{
    ffi::CStr,
    io,
    os::unix::io::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
};

/// Equivalent to [`openat`] with [`None`] passed for `dirfd`.
pub fn open(
    pathname: &CStr,
    flags: libc::c_int,
    mode: libc::mode_t,
) -> io::Result<OwnedFd>
{
    openat(None, pathname, flags, mode)
}

/// Call openat(2) with the given arguments.
///
/// If `dirfd` is [`None`], `AT_FDCWD` is passed.
pub fn openat(
    dirfd:    Option<BorrowedFd>,
    pathname: &CStr,
    flags:    libc::c_int,
    mode:     libc::mode_t,
) -> io::Result<OwnedFd>
{
    let dirfd = dirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);
    let flags = flags | libc::O_CLOEXEC;

    // SAFETY: path is NUL-terminated.
    let fd = unsafe { libc::openat(dirfd, pathname.as_ptr(), flags, mode) };

    if fd == -1 {
        return Err(io::Error::last_os_error());
    }

    // SAFETY: fd is a new, open file descriptor.
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}
