use {
    crate::{cstr::IntoCStr, stat},
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
pub fn fstatat<'a>(
    dirfd: Option<BorrowedFd>,
    pathname: impl IntoCStr<'a>,
    flags: libc::c_int,
) -> io::Result<stat>
{
    #[inline(never)]
    fn monomorphic(dirfd: libc::c_int, pathname: &CStr, flags: libc::c_int)
        -> io::Result<stat>
    {
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

    let dirfd = dirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);
    let pathname = pathname.into_cstr()?;
    monomorphic(dirfd, &pathname, flags)
}

/// Equivalent to [`mkdirat`] with [`None`] passed for `dirfd`.
pub fn mkdir<'a>(pathname: impl IntoCStr<'a>, mode: libc::mode_t)
    -> io::Result<()>
{
    mkdirat(None, pathname, mode)
}

/// Call mkdirat(2) with the given arguments.
///
/// If `dirfd` is [`None`], `AT_FDCWD` is passed.
pub fn mkdirat<'a>(
    dirfd: Option<BorrowedFd>,
    pathname: impl IntoCStr<'a>,
    mode: libc::mode_t,
) -> io::Result<()>
{
    #[inline(never)]
    fn monomorphic(dirfd: libc::c_int, pathname: &CStr, mode: libc::mode_t)
        -> io::Result<()>
    {
        // SAFETY: path is NUL-terminated.
        let result = unsafe {
            libc::mkdirat(dirfd, pathname.as_ptr(), mode)
        };

        if result == -1 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    let dirfd = dirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);
    let pathname = pathname.into_cstr()?;
    monomorphic(dirfd, &pathname, mode)
}

/// Equivalent to [`mknodat`] with [`None`] passed for `dirfd`.
pub fn mknod<'a>(
    pathname: impl IntoCStr<'a>,
    mode: libc::mode_t,
    dev: libc::dev_t,
) -> io::Result<()>
{
    mknodat(None, pathname, mode, dev)
}

/// Call mknodat(2) with the given arguments.
///
/// If `dirfd` is [`None`], `AT_FDCWD` is passed.
pub fn mknodat<'a>(
    dirfd: Option<BorrowedFd>,
    pathname: impl IntoCStr<'a>,
    mode: libc::mode_t,
    dev: libc::dev_t,
) -> io::Result<()>
{
    #[inline(never)]
    fn monomorphic(
        dirfd: libc::c_int,
        pathname: &CStr,
        mode: libc::mode_t,
        dev: libc::dev_t,
    ) -> io::Result<()>
    {
        // SAFETY: path is NUL-terminated.
        let result = unsafe {
            libc::mknodat(dirfd, pathname.as_ptr(), mode, dev)
        };

        if result == -1 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    }

    let dirfd = dirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);
    let pathname = pathname.into_cstr()?;
    monomorphic(dirfd, &pathname, mode, dev)
}
