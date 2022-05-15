use {
    super::{Mutator, StackRoot, UnsafeRef},
    std::{mem::{MaybeUninit, align_of, size_of}, ptr::NonNull, slice},
};

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
        /// Obtain the pre-allocated undef object.
        pub fn new<'h>(mutator: &Mutator<'h>, into: &StackRoot<'h>)
        {
            let object = mutator.heap.pre_alloc.undef();

            // SAFETY: Pre-allocated objects are always live.
            unsafe { into.set_unsafe(object) };
        }

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
        /// Obtain a pre-allocated Boolean object.
        pub fn new_from_bool<'h>(
            mutator: &Mutator<'h>,
            into: &StackRoot<'h>,
            value: bool,
        )
        {
            let object = if value {
                mutator.heap.pre_alloc.boolean_true()
            } else {
                mutator.heap.pre_alloc.boolean_false()
            };

            // SAFETY: Pre-allocated objects are always live.
            unsafe { into.set_unsafe(object) };
        }

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
        /// Create a new string object from bytes.
        ///
        /// The bytes must not include the terminating nul.
        /// This method will automatically add the terminating nul.
        pub fn new_from_bytes<'h>(
            mutator: &Mutator<'h>,
            into: &StackRoot<'h>,
            bytes: &[u8],
        )
        {
            // SAFETY: The passed function initializes the buffer.
            // SAFETY: The passed function does not act as a mutator.
            unsafe {
                Self::new_from_fn(mutator, into, bytes.len(), |buf| {
                    MaybeUninit::write_slice(buf, bytes);
                })
            }
        }

        /// Create a new string object and initialize it.
        ///
        /// The given function is called to initialize the string.
        /// The function must not write the terminating nul.
        /// This method will automatically add the terminating nul.
        ///
        /// # Safety
        ///
        /// When the given function returns, the buffer must be initialized.
        pub unsafe fn new_from_fn<'h>(
            mutator: &Mutator<'h>,
            into: &StackRoot<'h>,
            len: usize,
            init: impl FnOnce(&mut [MaybeUninit<u8>]),
        )
        {
            // NOTE: We do not prohibit init acting as a mutator;
            //       any code surrounding it must keep that in mind.

            // Skip allocation for empty string.
            if len == 0 {
                let object = mutator.heap.pre_alloc.string_empty();
                into.set_unsafe(object);
                return;
            }

            // Initialize string header.
            let description = Self::describe(len);
            let ptr = mutator.alloc(description.size);
            (description.init)(ptr);

            // Initialize string bytes.
            let string_ptr = ptr.as_ptr().cast::<String>();
            let bytes_ptr = (*string_ptr).bytes.as_mut_ptr();
            let bytes_ptr = bytes_ptr.cast::<MaybeUninit<u8>>();
            init(slice::from_raw_parts_mut(bytes_ptr, len));

            let object = UnsafeRef::new(ptr);
            into.set_unsafe(object);
        }

        /// Describe a string object.
        pub unsafe fn describe(len: usize)
            -> Description<impl FnOnce(NonNull<()>)>
        {
            Description{
                // TODO: Handle overflow.
                size: size_of::<Self>() + len,
                init: move |ptr| {
                    let ptr = ptr.as_ptr().cast::<Self>();
                    *ptr = Self{kind: Kind::String, len, bytes: []};
                },
            }
        }
    }
}
