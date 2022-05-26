use std::{ffi::CString, ptr::null_mut};

/// Null-terminated array of nul-terminated strings.
///
/// This can be used to prepare the arguments to execve(2),
/// which expects an array of pointers to C strings,
/// with a null pointer at the end of the array.
pub struct CStringVec
{
    inner: Vec<*mut libc::c_char>,
}

impl CStringVec
{
    /// Create an empty array.
    pub fn new() -> Self
    {
        Self{inner: vec![null_mut()]}
    }

    /// Append a nul-terminated string to the array.
    pub fn push(&mut self, cstr: CString)
    {
        self.inner.push(cstr.into_raw());

        // Swap terminating null and newly pushed cstr.
        let len = self.inner.len();
        self.inner.swap(len - 2, len - 1);
    }

    /// Obtain a pointer to the array.
    pub fn as_ptr(&self) -> *const *mut libc::c_char
    {
        self.inner.as_ptr()
    }
}

impl Drop for CStringVec
{
    fn drop(&mut self)
    {
        for cstr in &self.inner[0 .. self.inner.len() - 1] {
            // SAFETY: Pointer was obtained using CString::into_raw.
            drop(unsafe { CString::from_raw(*cstr) });
        }
    }
}

impl FromIterator<CString> for CStringVec
{
    fn from_iter<T>(iter: T) -> Self
        where T: IntoIterator<Item=CString>
    {
        let mut this = Self::new();
        for cstr in iter {
            this.push(cstr);
        }
        this
    }
}
