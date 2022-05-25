use {
    crate::retry_on_eintr,
    std::{
        ffi::{CStr, CString},
        io,
        os::unix::{ffi::OsStrExt, io::{AsRawFd, BorrowedFd}},
        path::Path,
    },
};

/// Equivalent to [`readlink`] with [`None`] passed for `dirfd`.
pub fn readlink(pathname: &Path) -> io::Result<CString>
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
pub fn readlinkat(dirfd: Option<BorrowedFd>, pathname: &Path)
    -> io::Result<CString>
{
    let dirfd = dirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);
    let pathname = CString::new(pathname.as_os_str().as_bytes())?;

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

/// Equivalent to [`symlink`] with [`None`] passed for `newdirfd`.
pub fn symlink(target: &CStr, linkpath: &Path) -> io::Result<()>
{
    symlinkat(target, None, linkpath)
}

/// Call symlinkat(2) with the given arguments.
///
/// If `newdirfd` is [`None`], `AT_FDCWD` is passed.
pub fn symlinkat(target: &CStr, newdirfd: Option<BorrowedFd>, linkpath: &Path)
    -> io::Result<()>
{
    let newdirfd = newdirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);
    let linkpath = CString::new(linkpath.as_os_str().as_bytes())?;

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
            let actual = readlinkat(None, Path::new(&symlink)).unwrap();
            assert_eq!(actual.as_bytes(), expected.as_bytes());
        }
    }
}
