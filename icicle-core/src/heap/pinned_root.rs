use {super::{BorrowRef, UnsafeRef}, std::fmt};

/// Pinned root to an object.
///
/// A pinned root ensures the object won't be garbage collected
/// and won't be moved in memory by the garbage collector.
/// A pinned root is very flexible; you can put it wherever,
/// as long as it doesn't outlive the heap that owns the object.
/// This flexibility comes at the cost of more overhead:
/// the [`Clone`] and [`Drop`] impls update a registry of all pinned roots,
/// and the garbage collector cannot move the object while pinned roots exist.
/// Please use [stack roots] if possible.
///
/// [stack roots]: `super::Mutator::with_stack_roots`
pub struct PinnedRoot<'h>
{
    // INVARIANT: The reference references a live object.
    inner: UnsafeRef<'h>,
}

// SAFETY: Registry of all pinned roots is updated synchronized.
unsafe impl<'h> Send for PinnedRoot<'h> { }
unsafe impl<'h> Sync for PinnedRoot<'h> { }

impl<'h> PinnedRoot<'h>
{
    /// Create a pinned root from a given reference.
    ///
    /// # Safety
    ///
    /// The referenced object must be live.
    pub (super) unsafe fn new(inner: UnsafeRef<'h>) -> Self
    {
        inner.heap().retain_pinned_root(inner);
        Self{inner}
    }
}

// SAFETY: Pinned roots always reference live objects.
unsafe impl<'h> BorrowRef<'h> for PinnedRoot<'h>
{
    fn borrow_ref(&self) -> UnsafeRef<'h>
    {
        self.inner
    }
}

impl<'h> Clone for PinnedRoot<'h>
{
    fn clone(&self) -> Self
    {
        BorrowRef::pin(self)
    }
}

impl<'h> Drop for PinnedRoot<'h>
{
    fn drop(&mut self)
    {
        let heap = self.heap();
        // SAFETY: Called from PinnedRoot::drop.
        unsafe { heap.release_pinned_root(self.inner); }
    }
}

impl<'h> fmt::Debug for PinnedRoot<'h>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        fmt::Debug::fmt(&self.inner, f)
    }
}
