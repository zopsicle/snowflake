use {
    super::{Heap, HeapId, block::block_header_at},
    std::{fmt, marker::PhantomData, ptr::NonNull},
};

/// Reference to an object owned by a heap.
///
/// This is the most basic type of reference to an object.
/// It provides no safety guarantees regarding object lifetimes.
/// References may be dangling, so care must be taken to ensure safe use.
/// Especially when references are used both
/// before and after garbage collection cycles.
///
/// Note that the garbage collector may move objects in memory.
/// When it does this, it will update references to objects.
/// This means that the [`Hash`], [`Ord`], and [`PartialOrd`] impls
/// may return different results across garbage collection cycles.
/// However, such updates do not use interior mutability inside [`UnsafeRef`],
/// so with normal Rust programming this will not cause any problems.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct UnsafeRef<'h>
{
    heap_id: HeapId<'h>,
    inner: NonNull<()>,
}

// SAFETY: Working with UnsafeRef already requires unsafe.
unsafe impl<'h> Send for UnsafeRef<'h> { }
unsafe impl<'h> Sync for UnsafeRef<'h> { }

impl<'h> UnsafeRef<'h>
{
    /// Create a reference from the address of an object.
    pub fn new(inner: NonNull<()>) -> Self
    {
        Self{heap_id: PhantomData, inner}
    }

    /// Obtain the address of the referenced object.
    pub fn as_ptr(self) -> NonNull<()>
    {
        self.inner
    }

    /// Obtain the heap the referenced object belongs to.
    ///
    /// # Safety
    ///
    /// The reference must reference a live object.
    pub unsafe fn heap(self) -> &'h Heap<'h>
    {
        let block_header = block_header_at(self);
        (*block_header).heap
    }
}

impl<'h> fmt::Debug for UnsafeRef<'h>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        fmt::Debug::fmt(&self.inner, f)
    }
}
