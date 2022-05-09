//! Working with Icicle values.

pub use self::{boolean::*, integer::*, string::*, undef::*, val::*};

use {
    scope_exit::ScopeExit,
    std::{
        alloc::{Layout, LayoutError, handle_alloc_error},
        fmt,
        hint::unreachable_unchecked,
        marker::PhantomData,
        mem::{align_of, forget, size_of},
        num::NonZeroU64,
        process::abort,
        ptr::NonNull,
        sync::atomic::{AtomicU32, Ordering::{Acquire, Relaxed, Release}, fence},
    },
};

mod boolean;
mod integer;
mod string;
mod undef;
mod val;

/// Reference-counted, dynamically-typed Icicle value.
///
/// A [`Value`] object can represent any Icicle value.
/// It contains enough information to determine the value's type.
/// Once a value has been constructed using one of the methods on [`Value`],
/// it can be conveniently inspected using [`borrow`][`Value::borrow`].
///
/// # Examples
///
/// ```
/// # use icicle_core::value::{Val, Value};
/// let value = Value::boolean_from_bool(true);
/// println!("{:?}", value.borrow());  // Boolean(true)
/// # assert_eq!(format!("{:?}", value.borrow()), "Boolean(true)");
/// ```
pub struct Value
{
    /// Off-heap or on-heap data, depending on least significant bit.
    ///
    /// Iff the least significant bit is set, the data is off-heap.
    /// Off-heap data consists of 60 bits of payload and 4 bits of tag.
    /// The structure of the payload depends on the value of the tag.
    ///
    /// On-heap data consists of a header and a payload.
    /// The header contains an atomic reference count and an "extra word".
    /// The extra word contains 28 bits of extra data and 4 bits of tag.
    /// The header is immediately followed by the on-heap payload.
    ///
    /// Off-heap data is never zero, because it is tagged.
    /// Likewise, pointers to on-heap data are never null.
    /// Using [`NonZeroU64`] we benefit from niche optimization.
    inner: NonZeroU64,

    /// Make sure we don't get implicit auto trait impls.
    ///
    /// As of nightly 2022-05-01, the would-be auto trait impls are fine.
    /// But a future version of Rust may introduce new auto trait impls
    /// which would be triggered if we only had the [`NonZeroU64`] above,
    /// and those might not be suitable for [`Value`].
    _phantom_data: PhantomData<*const ()>,
}

/// The different possible tags of an off-heap value.
///
/// For each tag we document the structure of associated payloads.
/// Payloads must be canonical: one value must not have multiple bit patterns.
/// So the values of any padding bytes must be explicitly defined.
mod off_heap_tag
{
    // NOTE: The least significant bit must be set in each tag.
    // NOTE: Tags must not use more than 4 bits each.

    /// The value is undef.
    ///
    /// The payload is 0.
    pub const UNDEF: u64 = 0b0001;

    /// The value is a Boolean.
    ///
    /// The payload is 1 for true, 0 for false.
    pub const BOOLEAN: u64 = 0b0011;

    /// The value is a 60-bit signed integer.
    ///
    /// The payload is the 60-bit signed integer.
    pub const INTEGER: u64 = 0b0101;

    /// The value is a string with a length no more than 6 bytes.
    ///
    /// Up to and including 6 most significant bytes store the string data.
    /// The remaining bytes of the payload are zero, and serve as padding.
    /// There must be at least one padding byte, the terminating nul.
    /// The length of the string in bytes is stored in
    /// the least significant nibble of the payload.
    pub const STRING: u64 = 0b0111;
}

/// The different possible tags of an on-heap value.
///
/// For each tag we document the structure of associated extra data.
/// The structure of the payloads is not documented here.
mod on_heap_tag
{
    // NOTE: Tags must not use more than 4 bits each.

    /// The value is a string.
    ///
    /// The extra data is zero.
    pub const STRING: u32 = 0b0000;
}

/// Header at the start of on-heap data.
///
/// The address of on-heap data never has the least significant bit set.
/// This is further ensured by specifying a minimum alignment greater than 1.
#[repr(align(8))]
struct OnHeapHeader
{
    /// The number of references to the value.
    ref_count: AtomicU32,

    /// Extra data and on-heap tag.
    ///
    /// See [`Value::inner`] for more information.
    extra_word: u32,
}

// We use synchronized interior mutability for on-heap data.
// Off-heap data does not use interior mutability.
unsafe impl Send for Value { }
unsafe impl Sync for Value { }

impl Value
{
    /// Create a value from off-heap data.
    ///
    /// The structure of the off-heap data must be valid
    /// as described in the documentation of the items in [`off_heap_tag`].
    unsafe fn from_off_heap(off_heap: u64) -> Self
    {
        debug_assert!(off_heap & 0b1 == 0b1, "Off-heap data must be tagged");
        Self{
            inner: NonZeroU64::new_unchecked(off_heap),
            _phantom_data: PhantomData,
        }
    }

    /// Create a value from on-heap data.
    ///
    /// The on-heap data must be properly initialized.
    /// This function will not modify the on-heap data.
    unsafe fn from_on_heap(on_heap: NonNull<OnHeapHeader>) -> Self
    {
        let on_heap = on_heap.as_ptr() as u64;
        debug_assert!(on_heap & 0b1 == 0b0, "On-heap data must be untagged");
        Self{
            inner: NonZeroU64::new_unchecked(on_heap),
            _phantom_data: PhantomData,
        }
    }

    /// Create a value from on-heap data.
    ///
    /// Allocate memory, initialize the on-heap header,
    /// and let `f` initialize the on-heap payload.
    /// `f` must return the extra word to be written to the header.
    ///
    /// If the size is too large, the relevant layout error is returned.
    /// If allocation fails, [`handle_alloc_error`] is called, as usual.
    ///
    /// This function is always inlined, to improve constant folding.
    /// That should eliminate the internal assertions in most cases.
    ///
    /// # Safety
    ///
    /// When `f` returns, the payload must be initialized,
    /// and the extra word must be valid as described in [`on_heap_tag`].
    ///
    /// The payload alignment must not be greater than `8`.
    #[inline(always)]
    unsafe fn new_on_heap(
        payload_size: usize,
        payload_align: usize,
        f: impl FnOnce(*mut ()) -> u32,
    ) -> Result<Self, LayoutError>
    {
        // Create allocation layout and check size is not too large.
        let layout = Layout::from_size_align(
            size_of::<OnHeapHeader>() + payload_size,
            align_of::<OnHeapHeader>(),
        )?;

        // If we allow payload alignment to be larger than header alignment,
        // we need to deal with padding bytes or address adjustment.
        // Both of those are annoying so we just do this instead.
        assert!(
            payload_align <= align_of::<OnHeapHeader>(),
            "Payload alignment is too large",
        );

        // With the std::alloc API, we must pass the layout when deallocating.
        // This is very annoying, so we just use malloc and free instead. :')
        let ptr = libc::malloc(layout.size());
        let Some(ptr) = NonNull::new(ptr)
            else { handle_alloc_error(layout); };

        // If anything below panics, deallocate the memory.
        let dealloc_guard = ScopeExit::new(|| {
            libc::free(ptr.as_ptr() as *mut libc::c_void);
        });

        // Initialize the payload.
        let ptr = ptr.cast::<OnHeapHeader>();
        let payload_ptr = ptr.as_ptr().add(1).cast::<()>();
        let extra_word = f(payload_ptr);

        // Initialize the header.
        let ref_count = AtomicU32::new(1);
        *ptr.as_ptr() = OnHeapHeader{ref_count, extra_word};

        forget(dealloc_guard);
        Ok(Self::from_on_heap(ptr))
    }

    fn is_off_heap(&self) -> bool
    {
        self.inner.get() & 0b1 == 0b1
    }

    fn is_on_heap(&self) -> bool
    {
        !self.is_off_heap()
    }

    /// Get the on-heap data, if the value is on-heap.
    fn get_on_heap(&self) -> Option<&OnHeapHeader>
    {
        if self.is_on_heap() {
            let inner = self.inner.get();
            let ptr = inner as *const OnHeapHeader;
            // SAFETY: The value is definitely on-heap.
            Some(unsafe { &*ptr })
        } else {
            None
        }
    }

    /// Call the correct `drop_on_heap_*` method and deallocate memory.
    ///
    /// The `drop_on_heap_*` methods drop any contained data.
    /// They do not deallocate the memory pointed to by the value;
    /// that is the responsibility of this method itself.
    ///
    /// This is not inlined, to reduce the size of the drop function.
    /// (The drop function is inlined all over the place.)
    ///
    /// # Safety
    ///
    /// `on_heap` must be allocated by [`new_on_heap`][`Self::new_on_heap`].
    /// After calling this method, `on_heap` can no longer be dereferenced.
    ///
    /// Each `drop_on_heap_*` method assumes that the value
    /// is on-heap and has a tag that it can handle.
    #[inline(never)]
    unsafe fn drop_on_heap(on_heap: &OnHeapHeader)
    {
        // Free any contained resources.
        let tag = on_heap.extra_word & 0b1111;
        match tag {
            on_heap_tag::STRING => Self::drop_on_heap_string(on_heap),
            _ => unreachable_unchecked(),
        }

        // Free the memory occupied by the on-heap data.
        libc::free(on_heap as *const OnHeapHeader as *mut libc::c_void);
    }
}

impl Clone for Value
{
    fn clone(&self) -> Self
    {
        if let Some(on_heap) = self.get_on_heap() {
            // Implementation taken from Arc::clone.
            let old_size = on_heap.ref_count.fetch_add(1, Relaxed);
            if old_size > i32::MAX as u32 {
                abort();
            }
        }
        Self{inner: self.inner, _phantom_data: PhantomData}
    }
}

impl Drop for Value
{
    fn drop(&mut self)
    {
        if let Some(on_heap) = self.get_on_heap() {
            // Implementation taken from Arc::drop.
            if on_heap.ref_count.fetch_sub(1, Release) != 1 {
                return;
            }
            fence(Acquire);

            // SAFETY: On-heap data is no longer going to be used.
            unsafe { Self::drop_on_heap(on_heap); }
        }
    }
}

impl fmt::Debug for Value
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        // For convenience, off-heap and on-heap values
        // are formatted with different letter case.
        // We explicitly *do not* want to use f.debug_tuple,
        // as that would insert noisy newlines with {:#?}.
        if self.is_off_heap() {
            write!(f, "Value({:#016X})", self.inner)
        } else {
            write!(f, "Value({:#016x})", self.inner)
        }
    }
}
