use {
    crate::cstr::IntoCStr,
    std::{
        ffi::{CString, OsString},
        io,
        os::unix::ffi::OsStringExt,
        path::PathBuf,
    },
};

/// Call mkdtemp(3) with the given arguments.
pub fn mkdtemp<'a>(template: impl IntoCStr<'a>) -> io::Result<PathBuf>
{
    #[inline(never)]
    fn monomorphic(template: CString) -> io::Result<PathBuf>
    {
        // CString::as_mut_ptr does not exist.
        let mut template = template.into_bytes_with_nul();

        // SAFETY: template is NUL-terminated.
        let ptr = unsafe {
            libc::mkdtemp(template.as_mut_ptr() as *mut libc::c_char)
        };

        if ptr.is_null() {
            return Err(io::Error::last_os_error());
        }

        // Remove NUL.
        template.pop();

        Ok(PathBuf::from(OsString::from_vec(template)))
    }

    let template = template.into_cstr()?;
    monomorphic(template.into_owned())
}
