use {
    super::{Heap, HeapId, block_header_at},
    std::{cell::Cell, fmt, marker::PhantomData, ptr::NonNull},
};

/* -------------------------------------------------------------------------- */
/*                                  BorrowRef                                 */
/* -------------------------------------------------------------------------- */

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

/* -------------------------------------------------------------------------- */
/*                                  PinnedRef                                 */
/* -------------------------------------------------------------------------- */

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

/* -------------------------------------------------------------------------- */
/*                                  UnsafeRef                                 */
/* -------------------------------------------------------------------------- */

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

/* -------------------------------------------------------------------------- */
/*                                 PinnedRoot                                 */
/* -------------------------------------------------------------------------- */

/// Pinned root to an object.
///
/// A pinned root ensures the object won't be garbage collected
/// and won't be moved in memory by the garbage collector.
/// A pinned root is very flexible; you can put it wherever,
/// as long as it doesn't outlive the heap that owns the object.
/// This flexibility comes at the cost of more overhead:
/// the [`Clone`] and [`Drop`] impls update a registry of all pinned roots,
/// and the garbage collector cannot move the object while pinned roots exist.
/// Please use [stack roots] or [pinned stack roots] if possible.
///
/// [stack roots]: `super::Mutator::with_stack_roots`
/// [pinned stack roots]: `super::Mutator::with_pinned_stack_root`
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

// SAFETY: Pinned roots always reference pinned objects.
unsafe impl<'h> PinnedRef<'h> for PinnedRoot<'h>
{
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

/* -------------------------------------------------------------------------- */
/*                                  StackRoot                                 */
/* -------------------------------------------------------------------------- */

/// Stack-allocated root to an object.
///
/// Stack roots are managed by [`Mutator::with_stack_roots`].
/// See the documentation on said method for more information.
///
/// [`Mutator::with_stack_roots`]: `super::Mutator::with_stack_roots`
pub struct StackRoot<'h>
{
    // The garbage collector modifies these references,
    // so we must enable interior mutability.
    // INVARIANT: The reference references a live object.
    inner: Cell<UnsafeRef<'h>>,
}

impl<'h> StackRoot<'h>
{
    /// Create a stack root from a given reference.
    ///
    /// # Safety
    ///
    /// The garbage collector must be aware of the stack root
    /// for as long as the stack root is being used.
    /// The referenced object must be live.
    pub (super) unsafe fn new(inner: UnsafeRef<'h>) -> Self
    {
        Self{inner: Cell::new(inner)}
    }

    /// Replace the stack root with the given reference.
    pub fn set(&self, val: impl BorrowRef<'h>)
    {
        self.inner.set(val.borrow_ref());
    }

    /// Replace the stack root with the given reference.
    ///
    /// # Safety
    ///
    /// The given reference must reference a live object.
    pub unsafe fn set_unsafe(&self, val: UnsafeRef<'h>)
    {
        self.inner.set(val);
    }
}

// SAFETY: Stack roots always reference live objects.
unsafe impl<'h> BorrowRef<'h> for StackRoot<'h>
{
    fn borrow_ref(&self) -> UnsafeRef<'h>
    {
        self.inner.get()
    }
}

/* -------------------------------------------------------------------------- */
/*                               PinnedStackRoot                              */
/* -------------------------------------------------------------------------- */

/// Pinned stack-allocated root to an object.
///
/// Pinned stack roots are managed by [`Mutator::with_pinned_stack_root`].
/// See the documentation on said method for more information.
///
/// [`Mutator::with_pinned_stack_root`]: `super::Mutator::with_pinned_stack_root`
pub struct PinnedStackRoot<'h>
{
    // NOTE: In contrast with StackRoot, we cannot use Cell here,
    //       because PinnedRef allows borrowing from the object.
    // INVARIANT: The reference references a live object.
    inner: UnsafeRef<'h>,
}

impl<'h> PinnedStackRoot<'h>
{
    /// Create a pinned stack root from a given reference.
    ///
    /// # Safety
    ///
    /// The garbage collector must be aware of the pinned stack root
    /// for as long as the pinned stack root is being used.
    /// The referenced object must be live.
    pub (super) unsafe fn new(inner: UnsafeRef<'h>) -> Self
    {
        Self{inner}
    }
}

// SAFETY: Pinned stack roots always reference live objects.
unsafe impl<'h> BorrowRef<'h> for PinnedStackRoot<'h>
{
    fn borrow_ref(&self) -> UnsafeRef<'h>
    {
        self.inner
    }
}

// SAFETY: Pinned stack roots always reference pinned objects.
unsafe impl<'h> PinnedRef<'h> for PinnedStackRoot<'h>
{
}

/* -------------------------------------------------------------------------- */
/*                                 Debug impls                                */
/* -------------------------------------------------------------------------- */

impl<'h> fmt::Debug for UnsafeRef<'h>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        fmt::Debug::fmt(&self.inner, f)
    }
}

impl<'h> fmt::Debug for PinnedRoot<'h>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        fmt::Debug::fmt(&self.inner, f)
    }
}
