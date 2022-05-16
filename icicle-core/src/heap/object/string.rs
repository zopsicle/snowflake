use {
    crate::istring::IStr,
    super::{
        super::{Mutator, StackRoot, UnsafeRef},
        CreateInfo,
        Kind,
        ObjectHeader,
        View,
    },
    std::{mem::{MaybeUninit, size_of}, ptr::NonNull, slice},
};

/// In-memory representation of string objects.
#[repr(C)]
pub struct String
{
    header: ObjectHeader,

    /// The number of bytes excluding the terminating nul.
    len: usize,

    /// The bytes that make up the string.
    bytes: [u8; 0 /* len + 1 */],
}

impl String
{
    /// Create info similar to [`Self::new_from_bytes`].
    pub (in super::super) unsafe fn create_info_from_bytes<'a>(bytes: &'a [u8])
        -> CreateInfo<impl 'a + FnOnce(NonNull<()>)>
    {
        return Self::create_info_from_fn(bytes.len(), |buf| {
            MaybeUninit::write_slice(buf, bytes);
        });
    }

    /// Create info similar to [`Self::new_from_fn`].
    pub (in super::super) unsafe fn create_info_from_fn<F>(len: usize, f: F)
        -> CreateInfo<impl FnOnce(NonNull<()>)>
        where F: FnOnce(&mut [MaybeUninit<u8>])
    {
        CreateInfo{

            // TODO: Handle overflow.
            size: size_of::<Self>() + len + 1,

            init: move |ptr| {
                let ptr = ptr.as_ptr().cast::<Self>();

                // Initialize string metadata.
                let header = ObjectHeader{kind: Kind::String};
                *ptr = Self{header, len, bytes: []};

                // Initialize string bytes.
                let bytes_ptr = (*ptr).bytes.as_mut_ptr();
                let bytes_ptr = bytes_ptr.cast::<MaybeUninit<u8>>();
                f(slice::from_raw_parts_mut(bytes_ptr, len));

                // Initialize terminating nul.
                *(*ptr).bytes.get_unchecked_mut(len) = 0;
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
            // Call init anyway as it may have side-effects.
            init(&mut []);
            let object = mutator.heap.pre_alloc.string_empty();
            into.set_unsafe(object);
            return;
        }

        // Initialize string header and terminating nul.
        let create_info = Self::create_info_from_fn(len, init);
        let ptr = mutator.alloc(create_info.size);
        (create_info.init)(ptr);

        let object = UnsafeRef::new(ptr);
        into.set_unsafe(object);
    }

    /// View a string object.
    pub fn view(&self) -> View
    {
        // SAFETY: len corresponds to the number of bytes.
        let bytes = unsafe {
            slice::from_raw_parts(self.bytes.as_ptr(), self.len + 1)
        };

        // SAFETY: We write the terminating nul during construction.
        let istr = unsafe { IStr::from_bytes_with_nul_unchecked(bytes) };

        View::String(istr)
    }
}
