use super::CompactRegion;

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
}

impl GcHeap
{
    pub fn new() -> Pin<Box<Self>>
    {
        let this = Self{
            _pinned: PhantomPinned,
            compact_regions: RefCell::new(HashSet::new()),
        };

        Box::into_pin(Box::new(this))
    }

    /// Make the garbage-collected heap a shared owner of a compact region.
    pub fn add_compact_region(&self, compact_region: Pin<Arc<CompactRegion>>)
    {
        let mut compact_regions = self.compact_regions.borrow_mut();
        compact_regions.insert(compact_region);
    }
}
