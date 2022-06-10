//! Working with C strings.

use std::{
    ffi::{CStr, CString, OsStr},
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::Path,
};

/// Convenient macro for creating a literal C string.
///
/// This macro automatically appends the terminating nul.
#[macro_export]
macro_rules! cstr
{
    ($lit:literal) => {
        unsafe {
            ::std::ffi::CStr::from_ptr(
                ::std::concat_bytes!($lit, b"\0").as_ptr().cast()
            )
        }
    };
}

/// Like [`cstr`], but convert the string into [`CString`].
#[macro_export]
macro_rules! cstring
{
    ($lit:literal) => {
        ::std::ffi::CString::from($crate::cstr!($lit))
    };
}

/// Like [`cstr`], but wrap the string in [`Cow::Borrowed`].
///
/// [`Cow::Borrowed`]: `std::borrow::Cow::Borrowed`
#[macro_export]
macro_rules! cstr_cow
{
    ($lit:literal) => {
        ::std::borrow::Cow::Borrowed($crate::cstr!($lit))
    };
}

/// Extra methods for [`CStr`].
pub trait CStrExt
{
    /// Join two paths like [`Path::join`][`std::path::Path::join`].
    fn join(&self, rhs: &CStr) -> CString;
}

impl CStrExt for CStr
{
    fn join(&self, rhs: &CStr) -> CString
    {
        let self_path = Path::new(OsStr::from_bytes(self.to_bytes()));
        let rhs_path = Path::new(OsStr::from_bytes(rhs.to_bytes()));
        let joined = self_path.join(rhs_path);
        let bytes = joined.into_os_string().into_vec();

        // SAFETY: Path::join does not insert NULs that weren't already there.
        unsafe { CString::from_vec_unchecked(bytes) }
    }

}
