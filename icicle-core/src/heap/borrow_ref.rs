use super::{Heap, PinnedRoot, UnsafeRef};

/// Trait for obtaining an [`UnsafeRef`] for safe use.
///
/// This trait is implemented by the various safe reference types.
/// It provides methods that can be used to safely work with objects,
/// by guaranteeing the reference is not dangling.
///
/// # Safety
///
/// The implementation of [`borrow_ref`][`Self::borrow_ref`]
/// must always return a reference to a live object.
/// Any overridden provided methods must be behaviorally
/// equivalent to their original provided implementations.
pub unsafe trait BorrowRef<'h>
{
    /// Return a reference to a live object.
    fn borrow_ref(&self) -> UnsafeRef<'h>;

    /// Obtain the heap the referenced object belongs to.
    fn heap(&self) -> &'h Heap<'h>
    {
        // SAFETY: borrow_ref returns a reference to a live object.
        unsafe { self.borrow_ref().heap() }
    }

    /// Create a pinned root to the object.
    fn pin(&self) -> PinnedRoot<'h>
    {
        // SAFETY: borrow_ref returns a reference to a live object.
        unsafe { PinnedRoot::new(self.borrow_ref()) }
    }
}

/// Trait for references to pinned objects.
///
/// While an object is pinned, the garbage collector will not move it.
/// This allows for safe borrowing of the contents of the object.
///
/// # Safety
///
/// In addition to the safety requirements of [`BorrowRef`],
/// the implementation must guarantee that [`borrow_ref`]
/// returns a reference to a pinned object.
///
/// [`borrow_ref`]: `BorrowRef::borrow_ref`
pub unsafe trait PinnedRef<'h>: BorrowRef<'h>
{
}
