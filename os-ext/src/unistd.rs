use {
    crate::retry_on_eintr,
    std::{
        ffi::{CString, OsString},
        io,
        os::unix::{ffi::{OsStrExt, OsStringExt}, io::{AsRawFd, BorrowedFd}},
        path::{Path, PathBuf},
    },
};

/// Equivalent to [`readlink`] with [`None`] passed for `dirfd`.
pub fn readlink(pathname: impl AsRef<Path>) -> io::Result<PathBuf>
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
pub fn readlinkat(dirfd: Option<BorrowedFd>, pathname: impl AsRef<Path>)
    -> io::Result<PathBuf>
{
    let dirfd = dirfd.map(|fd| fd.as_raw_fd()).unwrap_or(libc::AT_FDCWD);
    let pathname = CString::new(pathname.as_ref().as_os_str().as_bytes())?;

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

    Ok(PathBuf::from(OsString::from_vec(buf)))
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
            assert_eq!(actual, PathBuf::from(expected));
        }
    }
}
