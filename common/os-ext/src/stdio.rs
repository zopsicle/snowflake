use std::{ffi::CStr, io, os::unix::io::{AsRawFd, BorrowedFd}};

/// Call renameat2(2) with the given arguments.
///
/// If `olddirfd` or `newdirfd` is [`None`], `AT_FDCWD` is passed.
pub fn renameat2(
    olddirfd: Option<BorrowedFd>,
    oldpath:  &CStr,
    newdirfd: Option<BorrowedFd>,
    newpath:  &CStr,
    flags:    libc::c_uint,
) -> io::Result<()>
{
    let olddirfd = olddirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);
    let newdirfd = newdirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);

    // SAFETY: Paths are NUL-terminated.
    let result = unsafe {
        libc::renameat2(
            olddirfd, oldpath.as_ptr(),
            newdirfd, newpath.as_ptr(),
            flags,
        )
    };

    if result == -1 {
        return Err(io::Error::last_os_error());
    }

    Ok(())
}
