use std::{ffi::CString, io};

/// Call mkdtemp(3) with the given arguments.
pub fn mkdtemp(template: CString) -> io::Result<CString>
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

    // SAFETY: mkdtemp does not inject NULs.
    Ok(unsafe { CString::from_vec_with_nul_unchecked(template) })
}
