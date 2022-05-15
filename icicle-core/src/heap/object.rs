use std::{mem::{align_of, size_of}, ptr::{self, NonNull}};

/// Ensure that what embeds this is at least object-aligned.
#[repr(align(8))]
pub struct ObjectAlign;

/// Minimum required alignment for objects.
pub const OBJECT_ALIGN: usize = align_of::<ObjectAlign>();

/// In-memory representations of objects.
pub mod objects
{
    use super::*;

    /// Abstract description of an object.
    pub struct Description<F>
        where F: FnOnce(NonNull<()>)
    {
        /// How large the object is.
        pub size: usize,

        /// How to initialize the object.
        pub init: F,
    }

    /// Tag at the start of each object.
    #[allow(missing_docs)]
    #[repr(u32)]
    pub enum Kind
    {
        Undef,
        Boolean,
        String,
    }

    /// In-memory representation of undef objects.
    #[repr(C)]
    pub struct Undef
    {
        kind: Kind,
    }

    impl Undef
    {
        /// Describe an undef object.
        pub unsafe fn describe()
            -> Description<impl FnOnce(NonNull<()>)>
        {
            Description{
                size: size_of::<Self>(),
                init: |ptr| {
                    let ptr = ptr.as_ptr().cast::<Self>();
                    *ptr = Self{kind: Kind::Undef};
                },
            }
        }
    }

    /// In-memory representation of Boolean objects.
    #[repr(C)]
    pub struct Boolean
    {
        kind: Kind,
        value: bool,
    }

    impl Boolean
    {
        /// Describe a Boolean object.
        pub unsafe fn describe(value: bool)
            -> Description<impl FnOnce(NonNull<()>)>
        {
            Description{
                size: size_of::<Self>(),
                init: move |ptr| {
                    let ptr = ptr.as_ptr().cast::<Self>();
                    *ptr = Self{kind: Kind::Boolean, value};
                },
            }
        }
    }

    /// In-memory representation of string objects.
    #[repr(C)]
    pub struct String
    {
        kind: Kind,
        len: usize,
        bytes: [u8; 0],
    }

    impl String
    {
        /// Describe a string object.
        pub unsafe fn describe<'a>(data: &'a [u8])
            -> Description<impl 'a + FnOnce(NonNull<()>)>
        {
            Description{
                // TODO: Handle overflow.
                size: size_of::<Self>() + data.len(),
                init: |ptr| {
                    let ptr = ptr.as_ptr().cast::<Self>();
                    *ptr = Self{kind: Kind::String, len: data.len(), bytes: []};
                    ptr::copy_nonoverlapping(
                        data.as_ptr(),
                        (*ptr).bytes.as_mut_ptr(),
                        data.len(),
                    );
                },
            }
        }
    }
}
