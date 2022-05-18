use {
    super::{DEFAULT_BLOCK_SIZE, Block, BlockHeader, ObjectRef, object},
    std::{
        collections::HashSet,
        hash::{Hash, Hasher},
        marker::PhantomPinned,
        mem::{MaybeUninit, replace},
        pin::Pin,
        ptr,
        sync::{Arc, Mutex, MutexGuard},
    },
};

/// Compact region.
pub struct CompactRegion
{
    /// Each block stores a pointer to the compact region.
    _pinned: PhantomPinned,

    /// Compact regions are allocated into from multiple threads.
    inner: Mutex<Inner>,
}

struct Inner
{
    /// Shared ownership of other compact regions.
    compact_regions: HashSet<Pin<Arc<CompactRegion>>>,

    /// Block in which new allocations take place.
    allocation_block: Block,

    /// Blocks in which no more new allocations take place.
    retired_blocks: Vec<Block>,
}

impl CompactRegion
{
    /// Create a new compact region.
    pub fn new() -> Pin<Arc<Self>>
    {
        let mut arc = Arc::new_uninit();

        let allocation_block_header = BlockHeader::CompactRegion(arc.as_ptr());
        let allocation_block = Block::new(allocation_block_header);

        let inner = Inner{
            compact_regions: HashSet::new(),
            allocation_block,
            retired_blocks: Vec::new(),
        };

        let this = Self{
            _pinned: PhantomPinned,
            inner: Mutex::new(inner),
        };

        // SAFETY: There are no other shared owners.
        unsafe { Arc::get_mut_unchecked(&mut arc).write(this); }

        // SAFETY: We just initialized the payload.
        let arc = unsafe { arc.assume_init() };

        // SAFETY: We shall not move the compact region.
        unsafe { Pin::new_unchecked(arc) }
    }

    /// Lock the compact region's mutex and return a compactor.
    pub fn lock(&self) -> Compactor
    {
        let inner = self.inner.lock().unwrap();
        Compactor{compact_region: self, inner}
    }
}

/// Handle for creating objects in a compact region.
///
/// By grouping the object creation methods in a struct like this,
/// rather than providing them as methods on [`CompactRegion`] itself,
/// you can allocate multiple objects in succession with just one locking.
/// The compact region is unlocked again when the compactor is dropped.
pub struct Compactor<'a>
{
    compact_region: &'a CompactRegion,
    inner: MutexGuard<'a, Inner>,
}

impl<'a> Compactor<'a>
{
    /// Allocate and initialize memory for an undef object.
    pub fn new_undef(&mut self) -> ObjectRef
    {
        unsafe {
            let ptr = self.alloc(object::undef_size());
            object::undef_init(ptr)
        }
    }

    /// Allocate and initialize memory for a Boolean object.
    pub fn new_boolean_from_bool(&mut self, value: bool) -> ObjectRef
    {
        unsafe {
            let ptr = self.alloc(object::boolean_size());
            object::boolean_init_from_bool(ptr, value)
        }
    }

    /// Allocate and initialize memory for a string object.
    ///
    /// The string is initialized by the given function.
    ///
    /// # Safety
    ///
    ///  - The given function must initialize the entire buffer.
    ///  - The given function must not act as a mutator.
    pub unsafe fn new_string_from_fn<F>(&mut self, len: usize, f: F)
        -> ObjectRef
        where F: FnOnce(&mut [MaybeUninit<u8>])
    {
        let ptr = self.alloc(object::string_size(len));
        object::string_init_from_fn(ptr, len, f)
    }

    /// Allocate and initialize memory for an array object.
    ///
    /// The array is initialized by the given function.
    ///
    /// # Safety
    ///
    ///  - If the given function initializes an element to a compacted object,
    ///    the fiber must already have shared ownership of the compact region.
    ///  - The given function must initialize the entire array.
    ///  - The given function must not act as a mutator.
    pub unsafe fn new_array_from_fn<F>(&mut self, len: usize, f: F)
        -> ObjectRef
        where F: FnOnce(&mut [MaybeUninit<ObjectRef>])
    {
        let ptr = self.alloc(object::array_size(len));
        object::array_init_from_fn(ptr, len, f)
    }

    #[allow(missing_docs)]
    #[deprecated = "Slots cannot be compacted"]
    pub unsafe fn new_slot_from_object_ref(&mut self, object_ref: !)
        -> ObjectRef
    {
        object_ref
    }

    /// Allocate and initialize memory for a compact region handle object.
    ///
    /// If the given compact region is not this compact region,
    /// this compact region will obtain shared ownership of it.
    pub fn new_compact_region_handle_from_compact_region(
        &mut self,
        compact_region: Pin<Arc<CompactRegion>>,
    ) -> ObjectRef
    {
        unsafe {
            let cr: *const CompactRegion = &*compact_region;

            if &*compact_region != self.compact_region {
                self.inner.compact_regions.insert(compact_region);
            }

            let ptr = self.alloc(object::compact_region_handle_size());
            object::compact_region_handle_init_from_compact_region(ptr, cr)
        }
    }

    /// Allocate uninitialized memory for an object.
    unsafe fn alloc(&mut self, size: usize) -> *mut ()
    {
        if size > DEFAULT_BLOCK_SIZE {
            return self.alloc_large(size);
        }

        if let Some(ptr) = self.alloc_small_fast(size) {
            return ptr;
        }

        self.alloc_small_slow(size)
    }

    /// Allocate memory for an object in an ad-hoc block.
    #[inline(never)]
    unsafe fn alloc_large(&mut self, size: usize) -> *mut ()
    {
        let block_header = BlockHeader::CompactRegion(self.compact_region);
        let mut block = Block::with_size(size, block_header)
            .expect("Cannot create a block this large");

        let ptr = block.try_alloc(size)
            .expect("Ad-hoc block should be big enough");

        self.inner.retired_blocks.push(block);

        ptr
    }

    /// Try to allocate memory for an object in the allocation block.
    #[inline(always)]
    unsafe fn alloc_small_fast(&mut self, size: usize) -> Option<*mut ()>
    {
        self.inner.allocation_block.try_alloc(size)
    }

    /// Replace the allocation block with a new block
    /// and allocate memory for an object in it.
    #[inline(never)]
    unsafe fn alloc_small_slow(&mut self, size: usize) -> *mut ()
    {
        let block_header = BlockHeader::CompactRegion(self.compact_region);
        let mut block = Block::new(block_header);

        let ptr = block.try_alloc(size)
            .expect("New block should be big enough");

        let old_block = replace(&mut self.inner.allocation_block, block);
        self.inner.retired_blocks.push(old_block);

        ptr
    }
}

impl PartialEq for CompactRegion
{
    fn eq(&self, other: &CompactRegion) -> bool
    {
        ptr::eq(self, other)
    }
}

impl Eq for CompactRegion
{
}

impl Hash for CompactRegion
{
    fn hash<H>(&self, state: &mut H)
        where H: Hasher
    {
        ptr::hash(self, state)
    }
}
