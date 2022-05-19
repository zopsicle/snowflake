use {
    crate::{retry_on_eintr, stat},
    std::{
        ffi::CString,
        io,
        mem::MaybeUninit,
        os::unix::{ffi::OsStrExt, io::{AsRawFd, BorrowedFd}},
        path::Path,
    },
};

/// Call fstatat(2) with the given arguments.
///
/// If `dirfd` is [`None`], `AT_FDCWD` is passed.
pub fn fstatat(dirfd: Option<BorrowedFd>, pathname: &Path, flags: libc::c_int)
    -> io::Result<stat>
{
    let dirfd = dirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);
    let pathname = CString::new(pathname.as_os_str().as_bytes())?;

    let mut statbuf = MaybeUninit::uninit();

    retry_on_eintr(|| {
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
    })
}

/// Equivalent to [`mkdir`] with [`None`] passed for `dirfd`.
pub fn mkdir(pathname: &Path, mode: libc::mode_t) -> io::Result<()>
{
    mkdirat(None, pathname, mode)
}

/// Call mkdirat(2) with the given arguments.
///
/// If `dirfd` is [`None`], `AT_FDCWD` is passed.
pub fn mkdirat(dirfd: Option<BorrowedFd>, pathname: &Path, mode: libc::mode_t)
    -> io::Result<()>
{
    let dirfd = dirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);
    let path = CString::new(pathname.as_os_str().as_bytes())?;

    retry_on_eintr(|| {
        // SAFETY: path is NUL-terminated.
        let result = unsafe { libc::mkdirat(dirfd, path.as_ptr(), mode) };

        if result == -1 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    })
}
