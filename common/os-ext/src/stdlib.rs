use {
    crate::retry_on_eintr,
    std::{
        ffi::{CString, OsString},
        io,
        os::unix::ffi::{OsStrExt, OsStringExt},
        path::{Path, PathBuf},
    },
};

/// Call mkdtemp(3) with the given arguments.
pub fn mkdtemp(template: &Path) -> io::Result<PathBuf>
{
    let template = CString::new(template.as_os_str().as_bytes())?;

    // CString::as_mut_ptr does not exist.
    let mut template = template.into_bytes_with_nul();

    retry_on_eintr(|| {
        // SAFETY: template is NUL-terminated.
        let ptr = unsafe {
            libc::mkdtemp(template.as_mut_ptr() as *mut libc::c_char)
        };

        if ptr.is_null() {
            return Err(io::Error::last_os_error());
        }

        Ok(())
    })?;

    // Remove NUL.
    template.pop();

    Ok(PathBuf::from(OsString::from_vec(template)))
}
