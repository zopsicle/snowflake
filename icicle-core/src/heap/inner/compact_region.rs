use {
    super::{Block, BlockHeader},
    std::{
        collections::HashSet,
        hash::{Hash, Hasher},
        marker::PhantomPinned,
        pin::Pin,
        ptr,
        sync::{Arc, Mutex},
    },
};

/// Compact region.
pub struct CompactRegion
{
    /// Each block stores a pointer to the heap.
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
}

impl CompactRegion
{
    pub fn new() -> Pin<Arc<Self>>
    {
        let mut arc = Arc::new_uninit();

        let allocation_block_header = BlockHeader::CompactRegion(arc.as_ptr());
        let allocation_block = Block::new(allocation_block_header);

        let inner = Inner{
            compact_regions: HashSet::new(),
            allocation_block,
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
