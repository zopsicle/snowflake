use {
    crate::{gid_t, uid_t},
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
pub fn linkat(
    olddirfd: Option<BorrowedFd>,
    oldpath:  &CStr,
    newdirfd: Option<BorrowedFd>,
    newpath:  &CStr,
    flags:    libc::c_int,
) -> io::Result<()>
{
    let olddirfd = olddirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);
    let newdirfd = newdirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);

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
}

/// Call pipe2(2) with the given arguments.
pub fn pipe2(flags: libc::c_int) -> io::Result<(OwnedFd, OwnedFd)>
{
    let flags = flags | libc::O_CLOEXEC;

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
}

/// Equivalent to [`readlinkat`] with [`None`] passed for `dirfd`.
pub fn readlink(pathname: &CStr) -> io::Result<CString>
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
pub fn readlinkat(dirfd: Option<BorrowedFd>, pathname: &CStr)
    -> io::Result<CString>
{
    let dirfd = dirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);

    // NOTE: When changing the initial buffer size,
    //       adjust sizes of symlinks in testdata.
    let mut buf: Vec<u8> = Vec::with_capacity(256);

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

        // SAFETY: Symbolic links do not contain nuls.
        break Ok(unsafe { CString::from_vec_unchecked(buf) });
    }
}

/// Equivalent to [`symlinkat`] with [`None`] passed for `newdirfd`.
pub fn symlink(target: &CStr, linkpath: &CStr) -> io::Result<()>
{
    symlinkat(target, None, linkpath)
}

/// Call symlinkat(2) with the given arguments.
///
/// If `newdirfd` is [`None`], `AT_FDCWD` is passed.
pub fn symlinkat(
    target: &CStr,
    newdirfd: Option<BorrowedFd>,
    linkpath: &CStr,
) -> io::Result<()>
{
    let newdirfd = newdirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);

    // SAFETY: target and linkpath are NUL-terminated.
    let result = unsafe {
        libc::symlinkat(target.as_ptr(), newdirfd, linkpath.as_ptr())
    };

    if result == -1 {
        return Err(io::Error::last_os_error());
    }

    Ok(())
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
            let symlink = CString::new(format!("testdata/{}-byte-symlink", len)).unwrap();
            let actual = readlinkat(None, &symlink).unwrap();
            assert_eq!(actual.as_bytes(), expected.as_bytes());
        }
    }
}
