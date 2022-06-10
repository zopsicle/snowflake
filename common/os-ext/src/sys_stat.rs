use {
    crate::stat,
    std::{
        ffi::CStr,
        io,
        mem::MaybeUninit,
        os::unix::{io::{AsRawFd, BorrowedFd}},
    },
};

/// Call fstatat(2) with the given arguments.
///
/// If `dirfd` is [`None`], `AT_FDCWD` is passed.
pub fn fstatat(
    dirfd: Option<BorrowedFd>,
    pathname: &CStr,
    flags: libc::c_int,
) -> io::Result<stat>
{
    let dirfd = dirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);

    let mut statbuf = MaybeUninit::uninit();

    // SAFETY: path is NUL-terminated.
    let result = unsafe {
        libc::fstatat(
            dirfd,
            pathname.as_ptr(),
            statbuf.as_mut_ptr(),
            flags,
        )
    };

    if result == -1 {
        return Err(io::Error::last_os_error());
    }

    // SAFETY: fstatat initialized statbuf.
    Ok(unsafe { statbuf.assume_init() })
}

/// Equivalent to [`mkdirat`] with [`None`] passed for `dirfd`.
pub fn mkdir(pathname: &CStr, mode: libc::mode_t) -> io::Result<()>
{
    mkdirat(None, pathname, mode)
}

/// Call mkdirat(2) with the given arguments.
///
/// If `dirfd` is [`None`], `AT_FDCWD` is passed.
pub fn mkdirat(dirfd: Option<BorrowedFd>, pathname: &CStr, mode: libc::mode_t)
    -> io::Result<()>
{
    let dirfd = dirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);

    // SAFETY: path is NUL-terminated.
    let result = unsafe { libc::mkdirat(dirfd, pathname.as_ptr(), mode) };

    if result == -1 {
        return Err(io::Error::last_os_error());
    }

    Ok(())
}

/// Equivalent to [`mknodat`] with [`None`] passed for `dirfd`.
pub fn mknod(pathname: &CStr, mode: libc::mode_t, dev: libc::dev_t)
    -> io::Result<()>
{
    mknodat(None, pathname, mode, dev)
}

/// Call mknodat(2) with the given arguments.
///
/// If `dirfd` is [`None`], `AT_FDCWD` is passed.
pub fn mknodat(
    dirfd: Option<BorrowedFd>,
    pathname: &CStr,
    mode: libc::mode_t,
    dev: libc::dev_t,
) -> io::Result<()>
{
    let dirfd = dirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);

    // SAFETY: path is NUL-terminated.
    let result = unsafe { libc::mknodat(dirfd, pathname.as_ptr(), mode, dev) };

    if result == -1 {
        return Err(io::Error::last_os_error());
    }

    Ok(())
}
