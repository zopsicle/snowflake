use {super::super::{BorrowRef, UnsafeRef}, std::cell::Cell};

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
