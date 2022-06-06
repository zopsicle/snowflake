use std::{
    mem::{MaybeUninit, size_of},
    ptr::{NonNull, addr_of_mut},
    slice,
};

/* -------------------------------------------------------------------------- */
/*                                   Handles                                  */
/* -------------------------------------------------------------------------- */

/// Possibly dangling reference to an object.
#[derive(Clone, Copy)]
pub struct UnsafeHandle
{
    pub (super) inner: NonNull<Header>,
}

// Dereferencing unsafe handles is already unsafe.
unsafe impl Send for UnsafeHandle { }
unsafe impl Sync for UnsafeHandle { }

/// Handle to an object that frees it when dropped.
pub struct OwnedHandle
{
    pub inner: UnsafeHandle,
}

impl Drop for OwnedHandle
{
    fn drop(&mut self)
    {
        // SAFETY: Object was allocated using malloc.
        unsafe { libc::free(self.inner.inner.as_ptr().cast()); }
    }
}

/* -------------------------------------------------------------------------- */
/*                                   Header                                   */
/* -------------------------------------------------------------------------- */

pub struct Header
{
    r#type: Type,
}

enum Type
{
    Undef,
    String,
    Tuple,
}

/* -------------------------------------------------------------------------- */
/*                                    Undef                                   */
/* -------------------------------------------------------------------------- */

#[repr(C)]
pub struct Undef
{
    header: Header,
}

impl Undef
{
    pub fn size() -> usize
    {
        size_of::<Undef>()
    }

    pub unsafe fn init(ptr: *mut Self)
    {
        *ptr = Self{
            header: Header{
                r#type: Type::Undef,
            },
        };
    }
}

/* -------------------------------------------------------------------------- */
/*                                   String                                   */
/* -------------------------------------------------------------------------- */

#[repr(C)]
pub struct String
{
    header: Header,
    len:    usize,
    bytes:  [u8; 0 /* len */],
}

impl String
{
    pub fn size(len: usize) -> usize
    {
        size_of::<String>() + size_of::<u8>() * len
    }

    pub unsafe fn init<F>(ptr: *mut Self, len: usize, f: F)
        where F: FnOnce(&mut [MaybeUninit<u8>])
    {
        *ptr = Self{
            header: Header{
                r#type: Type::String,
            },
            len,
            bytes: [],
        };
        let ptr = addr_of_mut!((*ptr).bytes).cast();
        f(slice::from_raw_parts_mut(ptr, len))
    }
}

/* -------------------------------------------------------------------------- */
/*                                    Tuple                                   */
/* -------------------------------------------------------------------------- */

#[repr(C)]
pub struct Tuple
{
    header:   Header,
    len:      usize,
    elements: [UnsafeHandle; 0 /* len */],
}

impl Tuple
{
    pub fn size(len: usize) -> usize
    {
        size_of::<Tuple>() + size_of::<UnsafeHandle>() * len
    }

    pub unsafe fn init<F>(ptr: *mut Self, len: usize, f: F)
        where F: FnOnce(&mut [MaybeUninit<UnsafeHandle>])
    {
        *ptr = Self{
            header: Header{
                r#type: Type::Tuple,
            },
            len,
            elements: [],
        };
        let ptr = addr_of_mut!((*ptr).elements).cast();
        f(slice::from_raw_parts_mut(ptr, len))
    }
}
