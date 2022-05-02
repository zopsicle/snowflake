use {
    crate::retry_on_eintr,
    std::{
        ffi::CString,
        io,
        os::unix::{ffi::OsStrExt, io::{AsRawFd, BorrowedFd}},
        path::Path,
    },
};

/// Equivalent to [`mkdir`] with [`None`] passed for `dirfd`.
pub fn mkdir(pathname: impl AsRef<Path>, mode: libc::mode_t) -> io::Result<()>
{
    mkdirat(None, pathname, mode)
}

/// Call mkdirat(2) with the given arguments.
///
/// If `dirfd` is [`None`], `AT_FDCWD` is passed.
pub fn mkdirat(
    dirfd:    Option<BorrowedFd>,
    pathname: impl AsRef<Path>,
    mode:     libc::mode_t,
) -> io::Result<()>
{
    let dirfd = dirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);
    let path = CString::new(pathname.as_ref().as_os_str().as_bytes())?;

    retry_on_eintr(|| {
        // SAFETY: path is NUL-terminated.
        let result = unsafe { libc::mkdirat(dirfd, path.as_ptr(), mode) };

        if result == -1 {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    })
}
