//! Working with C strings.

use std::{
    borrow::Cow,
    ffi::{CStr, CString, NulError},
    os::unix::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
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

/// Trait for conversion into C strings.
pub trait IntoCStr<'a>
{
    /// Convert the string into a C string.
    ///
    /// The terminating nul will be added by this method.
    fn into_cstr(self) -> Result<Cow<'a, CStr>, NulError>;
}

impl<'a> IntoCStr<'a> for &'a CStr
{
    fn into_cstr(self) -> Result<Cow<'a, CStr>, NulError>
    {
        Ok(Cow::Borrowed(self))
    }
}

impl<'a> IntoCStr<'a> for CString
{
    fn into_cstr(self) -> Result<Cow<'a, CStr>, NulError>
    {
        Ok(Cow::Owned(self))
    }
}

impl<'a> IntoCStr<'a> for &'a CString
{
    fn into_cstr(self) -> Result<Cow<'a, CStr>, NulError>
    {
        <&CStr>::into_cstr(self)
    }
}

impl<'a> IntoCStr<'a> for &str
{
    fn into_cstr(self) -> Result<Cow<'a, CStr>, NulError>
    {
        CString::new(self).map(Cow::Owned)
    }
}

impl<'a> IntoCStr<'a> for String
{
    fn into_cstr(self) -> Result<Cow<'a, CStr>, NulError>
    {
        CString::new(self).map(Cow::Owned)
    }
}

impl<'a> IntoCStr<'a> for &String
{
    fn into_cstr(self) -> Result<Cow<'a, CStr>, NulError>
    {
        <&str>::into_cstr(self)
    }
}

impl<'a> IntoCStr<'a> for &Path
{
    fn into_cstr(self) -> Result<Cow<'a, CStr>, NulError>
    {
        CString::new(self.as_os_str().as_bytes()).map(Cow::Owned)
    }
}

impl<'a> IntoCStr<'a> for PathBuf
{
    fn into_cstr(self) -> Result<Cow<'a, CStr>, NulError>
    {
        CString::new(self.into_os_string().into_vec()).map(Cow::Owned)
    }
}

impl<'a> IntoCStr<'a> for &PathBuf
{
    fn into_cstr(self) -> Result<Cow<'a, CStr>, NulError>
    {
        <&Path>::into_cstr(self)
    }
}
