use {
    super::{CompactRegion, GcHeap},
    allocator_ext::AligningAllocator,
    std::{alloc::Global, default::default, mem::{MaybeUninit, size_of}, ptr},
    thiserror::Error,
};

/// Minimum alignment for blocks.
pub const BLOCK_ALIGN: usize = 4096;

/// Minimum alignment for objects.
pub const OBJECT_ALIGN: usize = 8;

/// Owning pointer to a block.
pub struct Block
{
    /// Pointer to the block.
    inner: Box<[MaybeUninit<u8>], AligningAllocator<Global, BLOCK_ALIGN>>,

    /// Offset at which to allocate the next object.
    ///
    /// The offset is in bytes and relative to inner.as_ptr().
    /// The block is full if offset >= min(inner.len(), BLOCK_ALIGN).
    offset: usize,
}

/// The heap that this block belongs to.
#[allow(missing_docs)]
pub enum BlockHeader
{
    GcHeap(*const GcHeap),
    CompactRegion(*const CompactRegion),
}

/// Returned when creating a block that would be too large.
#[derive(Debug, Error)]
#[error("Cannot create a block this large")]
pub struct BlockSizeError(());

impl Block
{
    /// Create a block with the default block size.
    ///
    /// The default block size is suitable for allocation of many small objects.
    /// The amount of memory allocated is precisely [`BLOCK_ALIGN`].
    pub fn new(header: BlockHeader) -> Self
    {
        let size = BLOCK_ALIGN - size_of::<BlockHeader>();
        Self::with_size(size, header).unwrap()
    }

    /// Create a block with the given size.
    ///
    /// The caller need not consider room for the block header;
    /// this method will add the size of the block header to the given size.
    pub fn with_size(size: usize, header: BlockHeader)
        -> Result<Self, BlockSizeError>
    {
        let header_size = size_of::<BlockHeader>();

        // Add space for the block header.
        let alloc_size = size.checked_add(header_size)
            .ok_or(BlockSizeError(()))?;

        let mut inner = Box::new_uninit_slice_in(alloc_size, default());

        // Initialize the block header.
        let ptr = inner.as_mut_ptr().cast::<BlockHeader>();
        // SAFETY: Size and alignment are sufficient.
        unsafe { ptr::write(ptr, header); }

        // Align the offset for the first object allocation.
        let offset = header_size.next_multiple_of(OBJECT_ALIGN);

        Ok(Self{inner, offset})
    }

    /// Attempt to allocate an object within the block.
    ///
    /// If there is insufficient room inside the block, returns [`None`].
    /// Otherwise returns a pointer to the uninitialized object.
    pub fn try_alloc(&mut self, size: usize) -> Option<*mut u8>
    {
        // Object must begin within first BLOCK_ALIGN bytes.
        if self.offset >= BLOCK_ALIGN {
            return None;
        }

        // Compute the offset just past the new object.
        let past_object = self.offset.checked_add(size)?;

        // Object must be fully contained within block.
        if past_object > self.inner.len() {
            return None;
        }

        // SAFETY: We just checked that this is in bounds.
        let ptr: *mut MaybeUninit<u8> = unsafe {
            self.inner.get_unchecked_mut(self.offset)
        };

        // There is no need to check for overflow here,
        // because a block that large wouldn't fit in memory.
        self.offset = next_multiple_of_power_of_two(past_object, OBJECT_ALIGN);

        Some(ptr.cast::<u8>())
    }
}

/// Round up `lhs` to the next multiple of `rhs`, which must be a power of two.
///
/// This is equivalent to [`usize::next_multiple_of`],
/// but LLVM generates better code for this function.
fn next_multiple_of_power_of_two(lhs: usize, rhs: usize) -> usize
{
    (lhs + rhs - 1) & !(rhs - 1)
}

#[cfg(test)]
mod tests
{
    use {
        super::*,
        proptest::{self as p, proptest},
        std::{mem::align_of, ptr::null},
    };

    const DUMMY_BLOCK_HEADER: BlockHeader = BlockHeader::GcHeap(null());

    #[test]
    fn block_align_exceeds_block_header_align()
    {
        assert!(
            BLOCK_ALIGN >= align_of::<BlockHeader>(),
            "Blocks must be aligned at least as much as block headers",
        );
    }

    #[test]
    fn can_create_zero_size_block()
    {
        Block::with_size(0, DUMMY_BLOCK_HEADER).unwrap();
    }

    proptest!
    {
        #[test]
        fn blocks_are_suitably_aligned(
            sizes in p::collection::vec(
                0usize ..= 20_000,
                p::collection::SizeRange::default(),
            ),
        )
        {
            // For this test we want each block to get a unique address.
            // We should therefore not free the blocks between the test cases.
            let blocks: Vec<Block> =
                sizes.into_iter()
                .map(|size| Block::with_size(size, DUMMY_BLOCK_HEADER))
                .map(Result::unwrap)
                .collect();
            for block in blocks {
                assert_eq!(block.inner.as_ptr() as usize % BLOCK_ALIGN, 0);
            }
        }

        #[test]
        fn next_multiple_of_power_of_two_agrees_with_next_multiple_of(
            lhs: usize,
            rhs_exp in 0 .. 8,
        )
        {
            let rhs = 1usize << rhs_exp;
            assert_eq!(
                next_multiple_of_power_of_two(lhs, rhs),
                lhs.next_multiple_of(rhs),
            );
        }
    }
}
