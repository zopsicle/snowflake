use {
    super::{
        DEFAULT_BLOCK_SIZE,
        Block,
        BlockHeader,
        CompactRegion,
        ObjectRef,
        object,
    },
    std::{
        cell::UnsafeCell,
        collections::HashSet,
        marker::PhantomPinned,
        mem::{MaybeUninit, replace},
        pin::Pin,
        sync::Arc,
    },
};

/// Fiber.
pub struct Fiber
{
    /// Each block stores a pointer to the fiber.
    _pinned: PhantomPinned,

    /// Interior mutability is required by the interface.
    inner: UnsafeCell<Inner>,
}

struct Inner
{
    /// Shared ownership of compact regions.
    compact_regions: HashSet<Pin<Arc<CompactRegion>>>,

    /// Block in which new allocations take place.
    allocation_block: Block,

    /// Blocks in which no more new allocations take place.
    retired_blocks: Vec<Block>,
}

impl Fiber
{
    /// Create a new fiber.
    pub fn new() -> Pin<Box<Self>>
    {
        let r#box = Box::new_uninit();

        let allocation_block_header = BlockHeader::Fiber(r#box.as_ptr());
        let allocation_block = Block::new(allocation_block_header);

        let inner = Inner{
            compact_regions: HashSet::new(),
            allocation_block: allocation_block,
            retired_blocks: Vec::new(),
        };

        let this = Self{
            _pinned: PhantomPinned,
            inner: UnsafeCell::new(inner),
        };

        Box::into_pin(Box::write(r#box, this))
    }

    /// Create an allocator for this fiber.
    ///
    /// # Safety
    ///
    /// No other allocator for this fiber may exist.
    unsafe fn allocator(&self) -> Allocator
    {
        let inner = &mut *self.inner.get();
        Allocator{fiber: self, inner}
    }

    /// Allocate and initialize memory for an undef object.
    ///
    /// # Safety
    ///
    /// Allocation may trigger garbage collection.
    pub unsafe fn new_undef(&self) -> ObjectRef
    {
        let mut allocator = self.allocator();
        let ptr = allocator.alloc(object::undef_size());
        object::undef_init(ptr)
    }

    /// Allocate and initialize memory for a Boolean object.
    ///
    /// # Safety
    ///
    /// Allocation may trigger garbage collection.
    pub unsafe fn new_boolean_from_bool(&mut self, value: bool) -> ObjectRef
    {
        let mut allocator = self.allocator();
        let ptr = allocator.alloc(object::boolean_size());
        object::boolean_init_from_bool(ptr, value)
    }

    /// Allocate and initialize memory for a string object.
    ///
    /// The string is initialized by the given function.
    ///
    /// # Safety
    ///
    ///  - Allocation may trigger garbage collection.
    ///  - The given function must initialize the entire buffer.
    ///  - The given function must not act as a mutator.
    pub unsafe fn new_string_from_fn<F>(&mut self, len: usize, f: F)
        -> ObjectRef
        where F: FnOnce(&mut [MaybeUninit<u8>])
    {
        let mut allocator = self.allocator();
        let ptr = allocator.alloc(object::string_size(len));
        object::string_init_from_fn(ptr, len, f)
    }

    /// Allocate and initialize memory for an array object.
    ///
    /// The array is initialized by the given function.
    ///
    /// # Safety
    ///
    ///  - Allocation may trigger garbage collection.
    ///  - If the given function initializes an element to a compacted object,
    ///    the fiber must already have shared ownership of the compact region.
    ///  - The given function must initialize the entire array.
    ///  - The given function must not act as a mutator.
    pub unsafe fn new_array_from_fn<F>(&mut self, len: usize, f: F)
        -> ObjectRef
        where F: FnOnce(&mut [MaybeUninit<ObjectRef>])
    {
        let mut allocator = self.allocator();
        let ptr = allocator.alloc(object::array_size(len));
        object::array_init_from_fn(ptr, len, f)
    }

    /// Allocate and initialize memory for a slot object.
    ///
    /// # Safety
    ///
    ///  - Allocation may trigger garbage collection.
    ///  - If the given reference references a compacted object,
    ///    the fiber must already have shared ownership of the compact region.
    pub unsafe fn new_slot_from_object_ref(&mut self, object_ref: ObjectRef)
        -> ObjectRef
    {
        let mut allocator = self.allocator();
        let ptr = allocator.alloc(object::slot_size());
        object::slot_init_from_object_ref(ptr, object_ref)
    }

    /// Allocate and initialize memory for a compact region handle object.
    ///
    /// This fiber will obtain shared ownership of the given compact region.
    ///
    /// # Safety
    ///
    ///  - Allocation may trigger garbage collection.
    pub unsafe fn new_compact_region_handle_from_compact_region(
        &mut self,
        compact_region: Pin<Arc<CompactRegion>>,
    ) -> ObjectRef
    {
        let mut allocator = self.allocator();

        let cr: *const CompactRegion = &*compact_region;

        allocator.inner.compact_regions.insert(compact_region);

        let ptr = allocator.alloc(object::compact_region_handle_size());
        object::compact_region_handle_init_from_compact_region(ptr, cr)
    }
}

struct Allocator<'a>
{
    fiber: &'a Fiber,
    inner: &'a mut Inner,
}

impl<'a> Allocator<'a>
{
    /// Allocate uninitialized memory for an object.
    unsafe fn alloc(&mut self, size: usize) -> *mut ()
    {
        if size > DEFAULT_BLOCK_SIZE {
            return self.alloc_large(size);
        }

        if let Some(ptr) = self.alloc_small_fast(size) {
            return ptr;
        }

        // TODO: Trigger garbage collection.

        self.alloc_small_slow(size)
    }

    /// Allocate memory for an object in an ad-hoc block.
    #[inline(never)]
    unsafe fn alloc_large(&mut self, size: usize) -> *mut ()
    {
        let block_header = BlockHeader::Fiber(self.fiber);
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
        let block_header = BlockHeader::Fiber(self.fiber);
        let mut block = Block::new(block_header);

        let ptr = block.try_alloc(size)
            .expect("New block should be big enough");

        let old_block = replace(&mut self.inner.allocation_block, block);
        self.inner.retired_blocks.push(old_block);

        ptr
    }
}
