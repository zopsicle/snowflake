use {
    super::{
        super::{Mutator, StackRoot, UnsafeRef},
        CreateInfo,
        Kind,
        ObjectHeader,
    },
    std::{mem::{MaybeUninit, size_of}, ptr::NonNull, slice},
};

/// In-memory representation of string objects.
#[repr(C)]
pub struct String
{
    header: ObjectHeader,
    len: usize,
    bytes: [u8; 0],
}

impl String
{
    pub (in super::super) unsafe fn create_info(len: usize)
        -> CreateInfo<impl FnOnce(NonNull<()>)>
    {
        CreateInfo{
            // TODO: Handle overflow.
            size: size_of::<Self>() + len,
            init: move |ptr| {
                let ptr = ptr.as_ptr().cast::<Self>();
                let header = ObjectHeader{kind: Kind::String};
                *ptr = Self{header, len, bytes: []};
            },
        }
    }

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
        let create_info = Self::create_info(len);
        let ptr = mutator.alloc(create_info.size);
        (create_info.init)(ptr);

        // Initialize string bytes.
        let string_ptr = ptr.as_ptr().cast::<String>();
        let bytes_ptr = (*string_ptr).bytes.as_mut_ptr();
        let bytes_ptr = bytes_ptr.cast::<MaybeUninit<u8>>();
        init(slice::from_raw_parts_mut(bytes_ptr, len));

        let object = UnsafeRef::new(ptr);
        into.set_unsafe(object);
    }
}
