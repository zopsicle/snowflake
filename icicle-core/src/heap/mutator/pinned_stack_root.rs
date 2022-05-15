use super::super::{BorrowRef, PinnedRef, UnsafeRef};

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
