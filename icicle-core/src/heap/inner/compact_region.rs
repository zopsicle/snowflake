use std::{
    collections::HashSet,
    hash::{Hash, Hasher},
    marker::PhantomPinned,
    pin::Pin,
    ptr,
    sync::{Arc, Mutex},
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
}

impl CompactRegion
{
    pub fn new() -> Pin<Arc<Self>>
    {
        let inner = Inner{
            compact_regions: HashSet::new(),
        };

        let this = Self{
            _pinned: PhantomPinned,
            inner: Mutex::new(inner),
        };

        let arc = Arc::new(this);

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
