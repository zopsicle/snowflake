use super::{Block, BlockHeader, CompactRegion};

use std::{
    cell::RefCell,
    collections::HashSet,
    marker::PhantomPinned,
    pin::Pin,
    sync::Arc,
};

/// Fiber.
pub struct Fiber
{
    /// Each block stores a pointer to the fiber.
    _pinned: PhantomPinned,

    /// Shared ownership of compact regions.
    compact_regions: RefCell<HashSet<Pin<Arc<CompactRegion>>>>,

    /// Block in which new allocations take place.
    allocation_block: RefCell<Block>,
}

impl Fiber
{
    pub fn new() -> Pin<Box<Self>>
    {
        let r#box = Box::new_uninit();

        let allocation_block_header = BlockHeader::Fiber(r#box.as_ptr());
        let allocation_block = Block::new(allocation_block_header);

        let this = Self{
            _pinned: PhantomPinned,
            compact_regions: RefCell::new(HashSet::new()),
            allocation_block: RefCell::new(allocation_block),
        };

        Box::into_pin(Box::write(r#box, this))
    }

    /// Make the fiber a shared owner of a compact region.
    pub fn add_compact_region(&self, compact_region: Pin<Arc<CompactRegion>>)
    {
        let mut compact_regions = self.compact_regions.borrow_mut();
        compact_regions.insert(compact_region);
    }
}
