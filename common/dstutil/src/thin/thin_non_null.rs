use {super::EnableThin, std::{marker::PhantomData, ptr::NonNull}};

/// Thin non-null pointer for a type that normally has fat pointers.
///
/// `ThinNonNull<T>` is logically equivalent to `NonNull<T>`
/// but is always represented by a single pointer.
pub struct ThinNonNull<T>
    where T: ?Sized
{
    inner: NonNull<()>,
    phantom: PhantomData<NonNull<T>>,
}

impl<T> Clone for ThinNonNull<T>
    where T: ?Sized
{
    fn clone(&self) -> Self
    {
        *self
    }
}

impl<T> Copy for ThinNonNull<T>
    where T: ?Sized
{
}

impl<T> ThinNonNull<T>
    where T: EnableThin + ?Sized
{
    /// Logically equivalent to [`NonNull::as_ref`].
    pub unsafe fn as_ref<'a>(&self) -> &'a T
    {
        &*T::fatten(self.inner.as_ptr())
    }
}

impl<T> From<&T> for ThinNonNull<T>
    where T: ?Sized
{
    fn from(other: &T) -> Self
    {
        let inner = NonNull::from(other).cast();
        Self{inner, phantom: PhantomData}
    }
}
