use super::{Block, BlockHeader, CompactRegion};

use std::{
    cell::RefCell,
    collections::HashSet,
    marker::PhantomPinned,
    pin::Pin,
    sync::Arc,
};

/// Garbage-collected heap.
pub struct GcHeap
{
    /// Each block stores a pointer to the heap.
    _pinned: PhantomPinned,

    /// Shared ownership of compact regions.
    compact_regions: RefCell<HashSet<Pin<Arc<CompactRegion>>>>,

    /// Block in which new allocations take place.
    allocation_block: RefCell<Block>,
}

impl GcHeap
{
    pub fn new() -> Pin<Box<Self>>
    {
        let r#box = Box::new_uninit();

        let allocation_block_header = BlockHeader::GcHeap(r#box.as_ptr());
        let allocation_block = Block::new(allocation_block_header);

        let this = Self{
            _pinned: PhantomPinned,
            compact_regions: RefCell::new(HashSet::new()),
            allocation_block: RefCell::new(allocation_block),
        };

        Box::into_pin(Box::write(r#box, this))
    }

    /// Make the garbage-collected heap a shared owner of a compact region.
    pub fn add_compact_region(&self, compact_region: Pin<Arc<CompactRegion>>)
    {
        let mut compact_regions = self.compact_regions.borrow_mut();
        compact_regions.insert(compact_region);
    }
}
