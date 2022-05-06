//! Nul-terminated strings, possibly with interior nuls.
//!
//! The [`IString`] and [`IStr`] types represent
//! string values as they appear in Icicle programs.
//! They are nul-terminated, like [`CString`] and [`CStr`],
//! but unlike those, nuls may also occur within the string.

use {
    std::{
        ffi::{self, CString, CStr},
        fmt,
        mem::transmute,
        ops::{Deref, DerefMut},
    },
    thiserror::Error,
};

/// Create a static nul-terminated string.
///
/// This macro will append a nul terminator to the string;
/// you should not add the nul terminator yourself.
#[macro_export]
macro_rules! istr
{
    ($str:literal) => {{
        let str: &'static [u8] = ::std::concat_bytes!($str, b"\0");
        // SAFETY: The string is nul-terminated.
        unsafe { $crate::istring::IStr::from_bytes_with_nul_unchecked(str) }
    }};
}

/// Owned, nul-terminated string, possibly with interior nuls.
pub struct IString
{
    // INVARIANT: Nul-terminated.
    bytes: Vec<u8>,
}

/// Borrowed, nul-terminated string, possibly with interior nuls.
#[repr(transparent)]
pub struct IStr
{
    // INVARIANT: Nul-terminated.
    bytes: [u8],
}

impl IString
{
    /// Create an empty string.
    pub fn new() -> Self
    {
        Self{bytes: vec![0]}
    }

    /// Append a byte to the string.
    pub fn push(&mut self, value: u8)
    {
        self.bytes.push(value);
        let len = self.bytes.len();
        // SAFETY: There are at least two elements (0 and value).
        unsafe { self.bytes.get_unchecked_mut(len - 2 .. len).reverse() };
    }

    /// Convert the string into a C string.
    ///
    /// If the string contains interior nuls, this method returns an error.
    pub fn into_cstring(self) -> Result<CString, ffi::FromVecWithNulError>
    {
        CString::from_vec_with_nul(self.bytes)
    }
}

impl IStr
{
    /// The number of bytes in the string, excluding the terminating nul.
    pub fn len(&self) -> usize
    {
        self.bytes.len() - 1
    }

    /// Create a string from a byte slice.
    ///
    /// The [`istr`] macro can be used to safely create a static string.
    pub fn from_bytes_with_nul(bytes: &[u8])
        -> Result<&Self, FromBytesWithNulError>
    {
        if bytes.last() == Some(&0) {
            Ok(unsafe { Self::from_bytes_with_nul_unchecked(bytes) })
        } else {
            Err(FromBytesWithNulError{_priv: ()})
        }
    }

    /// Create a string from a byte slice.
    ///
    /// The [`istr`] macro can be used to safely create a static string.
    ///
    /// # Safety
    ///
    /// The last byte in the byte slice must be zero.
    pub unsafe fn from_bytes_with_nul_unchecked(bytes: &[u8]) -> &Self
    {
        debug_assert_eq!(bytes.last(), Some(&0));
        transmute::<&[u8], &IStr>(bytes)
    }

    /// Create a string from a mutable byte slice.
    ///
    /// # Safety
    ///
    /// The last byte in the byte slice must be zero.
    pub unsafe fn from_bytes_with_nul_unchecked_mut(bytes: &mut [u8])
        -> &mut Self
    {
        debug_assert_eq!(bytes.last(), Some(&0));
        transmute::<&mut [u8], &mut IStr>(bytes)
    }

    /// The bytes in the string, excluding the terminating nul.
    pub fn as_bytes(&self) -> &[u8]
    {
        // SAFETY: len returns the correct length.
        unsafe { self.bytes.get_unchecked(0 .. self.len()) }
    }

    /// The mutable bytes in the string, excluding the terminating nul.
    pub fn as_bytes_mut(&mut self) -> &mut [u8]
    {
        // SAFETY: len returns the correct length.
        unsafe { self.bytes.get_unchecked_mut(0 .. self.len()) }
    }

    /// The bytes in the string, including the terminating nul.
    pub fn as_bytes_with_nul(&self) -> &[u8]
    {
        &self.bytes
    }

    /// The bytes in the string, as a C string.
    ///
    /// If the string contains interior nuls, this method returns an error.
    pub fn as_cstr(&self) -> Result<&CStr, ffi::FromBytesWithNulError>
    {
        CStr::from_bytes_with_nul(self.as_bytes_with_nul())
    }
}

impl Deref for IString
{
    type Target = IStr;

    fn deref(&self) -> &Self::Target
    {
        // SAFETY: The invariant guarantees nul-termination.
        unsafe { IStr::from_bytes_with_nul_unchecked(&self.bytes) }
    }
}

impl DerefMut for IString
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        // SAFETY: The invariant guarantees nul-termination.
        unsafe { IStr::from_bytes_with_nul_unchecked_mut(&mut self.bytes) }
    }
}

impl fmt::Debug for IString
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        <IStr as fmt::Debug>::fmt(self, f)
    }
}

impl fmt::Debug for IStr
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        write!(f, "\"{}\"", self.as_bytes().escape_ascii())
    }
}

/// Returned by [`IStr::from_bytes_with_nul`]
/// when the string is missing a nul terminator.
#[derive(Debug, Eq, Error, PartialEq)]
#[error("String is missing nul terminator")]
pub struct FromBytesWithNulError
{
    _priv: (),
}

#[cfg(test)]
mod tests
{
    use {super::*, proptest::proptest};

    proptest!
    {
        #[test]
        fn push(mut bytes: Vec<u8>)
        {
            let mut istring = IString::new();
            for &byte in &bytes {
                istring.push(byte);
            }
            bytes.push(0);
            assert_eq!(istring.as_bytes_with_nul(), bytes);
        }

        #[test]
        fn len(mut bytes: Vec<u8>)
        {
            bytes.push(0);
            let istr = IStr::from_bytes_with_nul(&bytes).unwrap();
            assert_eq!(istr.len(), bytes.len() - 1);
        }

        #[test]
        fn as_bytes(mut bytes: Vec<u8>)
        {
            bytes.push(0);
            let istr = IStr::from_bytes_with_nul(&bytes).unwrap();
            assert_eq!(istr.as_bytes(), &bytes[0 .. bytes.len() - 1]);
        }

        #[test]
        fn as_bytes_with_nul(mut bytes: Vec<u8>)
        {
            bytes.push(0);
            let istr = IStr::from_bytes_with_nul(&bytes).unwrap();
            assert_eq!(istr.as_bytes_with_nul(), &bytes);
        }

        #[test]
        fn as_cstr(mut bytes: Vec<u8>)
        {
            bytes.push(0);
            let istr = IStr::from_bytes_with_nul(&bytes).unwrap();
            assert_eq!(istr.as_cstr(), CStr::from_bytes_with_nul(&bytes));
        }
    }
}
