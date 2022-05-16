use {
    super::{GcHeap, UnsafeRef, object::{ObjectAlign, OBJECT_ALIGN}},
    std::{
        alloc::{Layout, alloc, dealloc, handle_alloc_error},
        mem::size_of,
        ptr::{self, NonNull, from_exposed_addr},
    },
};

/// Alignment for blocks.
///
/// The address of every block is a multiple of this value.
pub const BLOCK_ALIGN: usize = 4096;

/// Default size for blocks.
///
/// Blocks larger than this are only needed for very large objects.
pub const DEFAULT_BLOCK_SIZE: usize = BLOCK_ALIGN - size_of::<BlockHeader>();

/// Owned block.
///
/// A block is a region of memory that stores a [block header]
/// followed immediately by a sequence of zero or more objects.
/// Objects may be of different sizes, but are all aligned to [`OBJECT_ALIGN`].
///
/// Objects are always allocated within blocks; never outside of blocks.
/// Objects are always located in the first [`BLOCK_ALIGN`] bytes of the block
/// (although any non-first bytes of the last object in the block might not).
/// Using this guarantee, the block header can be found by rounding down
/// the address of the object to the nearest multiple of [`BLOCK_ALIGN`].
///
/// Blocks also keep track of where to allocate new objects.
/// They contain a pointer that can be bumped to perform allocations.
/// The [`try_alloc`] method bumps this pointer and returns its old value.
///
/// [block header]: `BlockHeader`
/// [`try_alloc`]: `Self::try_alloc`
pub struct Block<'h>
{
    /// The address of the block.
    ///
    /// The block always begins with a block header,
    /// so we use this type for convenience.
    /// The objects follow the block header.
    ptr: NonNull<BlockHeader<'h>>,

    /// The number of bytes that make up the block.
    len: usize,

    /// The `ptr`-relative offset of the next object to be allocated.
    ///
    /// This offset is in bytes and a multiple of [`OBJECT_ALIGN`].
    offset: usize,
}

impl<'h> Block<'h>
{
    /// Allocate a block with the default block size.
    pub fn new(heap: &'h GcHeap<'h>) -> Self
    {
        Self::with_capacity(heap, DEFAULT_BLOCK_SIZE)
    }

    /// Allocate a block with a given block size.
    ///
    /// The given size must not include the size of the block header;
    /// this method will add the necessary block header size.
    pub fn with_capacity(heap: &'h GcHeap<'h>, cap: usize) -> Self
    {
        // Reserve space for the block header.
        let len = cap.checked_add(size_of::<BlockHeader>())
            .expect("Cannot allocate a block this large");

        // Create layout for allocation.
        let layout = Layout::from_size_align(len, BLOCK_ALIGN)
            .expect("Cannot allocate a block this large");

        // Allocate memory for the block.
        // SAFETY: len is non-zero, because we added the size of BlockHeader.
        let ptr = unsafe { alloc(layout) };
        let Some(ptr) = NonNull::new(ptr)
            else { handle_alloc_error(layout) };

        // Write the block header.
        let ptr = ptr.cast::<BlockHeader>();
        let header = BlockHeader{heap, _object_align: ObjectAlign};
        // SAFETY: ptr points to fresh memory that is suitably aligned.
        unsafe { ptr::write(ptr.as_ptr(), header); }

        // Compute the offset, which is right after the block header.
        let offset = size_of::<BlockHeader>();

        Self{ptr, len, offset}
    }

    /// The block header for this block.
    pub fn block_header(&self) -> &BlockHeader<'h>
    {
        // SAFETY: The memory is allocated and initialized.
        unsafe { &*self.ptr.as_ptr() }
    }

    /// Allocate memory for an object within the block.
    ///
    /// If the block has insufficient space for the object,
    /// this method returns [`None`] and nothing changes.
    ///
    /// Note that the caller must initialize the allocated memory
    /// before the garbage collector next traverses this block.
    pub fn try_alloc(&mut self, size: usize) -> Option<NonNull<()>>
    {
        // Make sure the *next* allocation will also be aligned.
        let size = size.checked_next_multiple_of(OBJECT_ALIGN)
            .expect("Cannot allocate an object this large");

        // Check that the object will start within BLOCK_ALIGN bytes.
        if self.offset >= BLOCK_ALIGN {
            return None;
        }

        // Compute the offset for the next allocation.
        let new_offset = self.offset.checked_add(size)?;

        // Check that the object fits in the block.
        if new_offset > self.len {
            return None;
        }

        // Compute the pointer to the new object.
        // SAFETY: The pointer is within the allocated block.
        let ptr = unsafe { self.ptr.as_ptr().cast::<u8>().add(self.offset) };
        let ptr = unsafe { NonNull::new_unchecked(ptr) };

        // Update the offset.
        self.offset = new_offset;

        Some(ptr.cast())
    }
}

// SAFETY: Block::drop does not access heap.
unsafe impl<#[may_dangle] 'h> Drop for Block<'h>
{
    fn drop(&mut self)
    {
        // SAFETY: This matches Block::with_capacity.
        let layout = unsafe {
            Layout::from_size_align_unchecked(self.len, BLOCK_ALIGN)
        };

        // SAFETY: ptr and layout come from with_capacity.
        unsafe { dealloc(self.ptr.as_ptr().cast(), layout) }
    }
}

/// Data at the start of each block.
pub struct BlockHeader<'h>
{
    /// The heap to which this block belongs.
    pub heap: &'h GcHeap<'h>,

    /// Ensure sufficient alignment for objects.
    ///
    /// Because objects immediately follow the block header,
    /// block headers must be at least aligned like objects.
    _object_align: ObjectAlign,
}

/// Compute the address of the block header
/// of the block that contains `object`.
pub fn block_header_at(object: UnsafeRef) -> *const BlockHeader
{
    let object = object.as_ptr().as_ptr();
    // Discard all the bits that vary for this block.
    // For example, 0b00010000 - 1 = 0b00001111.
    let mask = BLOCK_ALIGN - 1;
    from_exposed_addr(object.expose_addr() & !mask)
}

#[cfg(test)]
mod tests
{
    use {
        super::*,
        proptest::proptest,
        std::{mem::align_of, ptr::{NonNull, from_exposed_addr_mut}},
    };

    #[test]
    fn block_align_properties()
    {
        // Alignments must always be powers of two.
        assert!(BLOCK_ALIGN.is_power_of_two());

        // Make sure we can safely access the block header.
        assert!(BLOCK_ALIGN >= align_of::<BlockHeader>());
    }

    proptest!
    {
        #[test]
        fn block_with_capacity_aligns_properly(cap in 0usize .. 12_000)
        {
            GcHeap::with(|heap| {
                let block = Block::with_capacity(heap, cap);
                let ptr: *const _ = block.block_header();
                assert_eq!(ptr.expose_addr() % BLOCK_ALIGN, 0);
            });
        }

        #[test]
        fn block_header_at_returns_block_start(addr in 1usize ..)
        {
            // Turn the address into a reference.
            let ptr = from_exposed_addr_mut::<()>(addr);
            let object = UnsafeRef::new(NonNull::new(ptr).unwrap());

            // Check that block_header_at returns a suitable object.
            let block_addr = block_header_at(object).expose_addr();
            assert_eq!(block_addr % BLOCK_ALIGN, 0);
            assert!(addr - block_addr < BLOCK_ALIGN);
        }
    }
}
