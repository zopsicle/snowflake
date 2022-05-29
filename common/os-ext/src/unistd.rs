use {
    crate::{cstr::IntoCStr, gid_t, retry_on_eintr, uid_t},
    std::{
        ffi::{CStr, CString},
        io,
        os::unix::io::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
    },
};

/// Call getgid(2).
pub fn getgid() -> gid_t
{
    // SAFETY: This is always safe.
    unsafe { libc::getgid() }
}

/// Call getuid(2).
pub fn getuid() -> uid_t
{
    // SAFETY: This is always safe.
    unsafe { libc::getuid() }
}

/// Call linkat(2) with the given arguments.
///
/// If `olddirfd` or `newdirfd` is [`None`], `AT_FDCWD` is passed.
pub fn linkat<'a, 'b>(
    olddirfd: Option<BorrowedFd>,
    oldpath:  impl IntoCStr<'a>,
    newdirfd: Option<BorrowedFd>,
    newpath:  impl IntoCStr<'b>,
    flags:    libc::c_int,
) -> io::Result<()>
{
    #[inline(never)]
    fn monomorphic(
        olddirfd: libc::c_int,
        oldpath:  &CStr,
        newdirfd: libc::c_int,
        newpath:  &CStr,
        flags:    libc::c_int,
    ) -> io::Result<()>
    {
        retry_on_eintr(|| {
            // SAFETY: Paths are NUL-terminated.
            let result = unsafe {
                libc::linkat(
                    olddirfd, oldpath.as_ptr(),
                    newdirfd, newpath.as_ptr(),
                    flags,
                )
            };

            if result == -1 {
                return Err(io::Error::last_os_error());
            }

            Ok(())
        })
    }

    let olddirfd = olddirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);
    let newdirfd = newdirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);
    let oldpath = oldpath.into_cstr()?;
    let newpath = newpath.into_cstr()?;
    monomorphic(olddirfd, &oldpath, newdirfd, &newpath, flags)
}

/// Call pipe2(2) with the given arguments.
pub fn pipe2(flags: libc::c_int) -> io::Result<(OwnedFd, OwnedFd)>
{
    let flags = flags | libc::O_CLOEXEC;

    retry_on_eintr(|| {
        let mut pipefd = [-1; 2];
        // SAFETY: pipefd is sufficiently large.
        let result = unsafe { libc::pipe2(pipefd.as_mut_ptr(), flags) };

        if result == -1 {
            return Err(io::Error::last_os_error());
        }

        Ok((
            // SAFETY: These file descriptors are fresh.
            unsafe { OwnedFd::from_raw_fd(pipefd[0]) },
            unsafe { OwnedFd::from_raw_fd(pipefd[1]) }
        ))
    })
}

/// Equivalent to [`readlinkat`] with [`None`] passed for `dirfd`.
pub fn readlink<'a>(pathname: impl IntoCStr<'a>) -> io::Result<CString>
{
    readlinkat(None, pathname)
}

/// Call readlinkat(2) with the given arguments.
///
/// If `dirfd` is [`None`], `AT_FDCWD` is passed.
///
/// readlinkat(2) truncates the target if it does not fit into the buffer.
/// When this happens, the wrapper function automatically retries the call
/// with a bigger buffer, until it fits.
pub fn readlinkat<'a>(
    dirfd: Option<BorrowedFd>,
    pathname: impl IntoCStr<'a>,
) -> io::Result<CString>
{
    #[inline(never)]
    fn monomorphic(dirfd: libc::c_int, pathname: &CStr) -> io::Result<CString>
    {
        // NOTE: When changing the initial buffer size,
        //       adjust sizes of symlinks in testdata.
        let mut buf: Vec<u8> = Vec::with_capacity(256);

        retry_on_eintr(|| {
            loop {
                // SAFETY: pathname is NUL-terminated, buffer size is correct.
                let len = unsafe {
                    libc::readlinkat(
                        dirfd,
                        pathname.as_ptr(),
                        buf.as_mut_ptr() as *mut libc::c_char,
                        buf.capacity(),
                    )
                };

                if len == -1 {
                    return Err(io::Error::last_os_error());
                }

                // SAFETY: readlinkat(2) wrote this many bytes.
                unsafe { buf.set_len(len as usize); }

                if buf.len() == buf.capacity() {
                    // There may have been a truncation.
                    // Grow the buffer and try again.
                    buf.reserve(1);
                    continue;
                }

                buf.shrink_to_fit();
                break Ok(());
            }
        })?;

        // SAFETY: Symbolic links do not contain nuls.
        Ok(unsafe { CString::from_vec_unchecked(buf) })
    }

    let dirfd = dirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);
    let pathname = pathname.into_cstr()?;
    monomorphic(dirfd, &pathname)
}

/// Equivalent to [`symlinkat`] with [`None`] passed for `newdirfd`.
pub fn symlink<'a>(target: &CStr, linkpath: impl IntoCStr<'a>)
    -> io::Result<()>
{
    symlinkat(target, None, linkpath)
}

/// Call symlinkat(2) with the given arguments.
///
/// If `newdirfd` is [`None`], `AT_FDCWD` is passed.
pub fn symlinkat<'a>(
    target: &CStr,
    newdirfd: Option<BorrowedFd>,
    linkpath: impl IntoCStr<'a>,
) -> io::Result<()>
{
    #[inline(never)]
    fn monomorphic(target: &CStr, newdirfd: libc::c_int, linkpath: &CStr)
        -> io::Result<()>
    {
        retry_on_eintr(|| {
            // SAFETY: target and linkpath are NUL-terminated.
            let result = unsafe {
                libc::symlinkat(target.as_ptr(), newdirfd, linkpath.as_ptr())
            };

            if result == -1 {
                return Err(io::Error::last_os_error());
            }

            Ok(())
        })
    }

    let newdirfd = newdirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);
    let linkpath = linkpath.into_cstr()?;
    monomorphic(target, newdirfd, &linkpath)
}


#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn readlinkat_loop()
    {
        for len in [10, 255, 256, 257, 512] {
            let expected: String = "0123456789".chars().cycle().take(len).collect();
            let symlink = format!("testdata/{}-byte-symlink", len);
            let actual = readlinkat(None, symlink).unwrap();
            assert_eq!(actual.as_bytes(), expected.as_bytes());
        }
    }
}
