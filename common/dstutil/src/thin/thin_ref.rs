use {super::{EnableThin, ThinNonNull}, std::{marker::PhantomData, ops::Deref}};

/// Thin reference for a type that normally has fat pointers.
///
/// `ThinRef<'a, T>` is logically equivalent to `&'a T`
/// but is always represented by a single pointer.
pub struct ThinRef<'a, T>
    where T: ?Sized
{
    inner: ThinNonNull<T>,
    phantom: PhantomData<&'a T>,
}

unsafe impl<'a, T> Send for ThinRef<'a, T>
    where T: ?Sized + Sync
{
}

impl<'a, T> Clone for ThinRef<'a, T>
    where T: ?Sized
{
    fn clone(&self) -> Self
    {
        *self
    }
}

impl<'a, T> Copy for ThinRef<'a, T>
    where T: ?Sized
{
}

impl<'a, T> ThinRef<'a, T>
    where T: ?Sized
{
    /// Create a thin reference from a fat reference.
    pub fn new(r#ref: &'a T) -> Self
    {
        let inner = ThinNonNull::from(r#ref);
        Self{inner, phantom: PhantomData}
    }
}

impl<'a, T> Deref for ThinRef<'a, T>
    where T: EnableThin + ?Sized
{
    type Target = T;

    fn deref(&self) -> &Self::Target
    {
        unsafe { self.inner.as_ref() }
    }
}
