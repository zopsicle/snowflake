use {
    crate::cstr::IntoCStr,
    std::{ffi::CStr, io, os::unix::io::{AsRawFd, BorrowedFd}},
};

/// Call renameat2(2) with the given arguments.
///
/// If `olddirfd` or `newdirfd` is [`None`], `AT_FDCWD` is passed.
pub fn renameat2<'a, 'b>(
    olddirfd: Option<BorrowedFd>,
    oldpath:  impl IntoCStr<'a>,
    newdirfd: Option<BorrowedFd>,
    newpath:  impl IntoCStr<'b>,
    flags:    libc::c_uint,
) -> io::Result<()>
{
    #[inline(never)]
    fn monomorphic(
        olddirfd: libc::c_int,
        oldpath:  &CStr,
        newdirfd: libc::c_int,
        newpath:  &CStr,
        flags:    libc::c_uint,
    ) -> io::Result<()>
    {
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

    let olddirfd = olddirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);
    let newdirfd = newdirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);
    let oldpath = oldpath.into_cstr()?;
    let newpath = newpath.into_cstr()?;
    monomorphic(olddirfd, &oldpath, newdirfd, &newpath, flags)
}
