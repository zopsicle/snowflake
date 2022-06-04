use std::{cell, ops::{Deref, DerefMut}};

/// Like [`RefCell`][`cell::RefCell`], but without
/// runtime checks when debug assertions are disabled.
pub struct UnsafeRefCell<T>
    where T: ?Sized
{
    #[cfg(debug_assertions)]
    inner: cell::RefCell<T>,

    #[cfg(not(debug_assertions))]
    inner: cell::UnsafeCell<T>,
}

pub struct Ref<'a, T>
    where T: 'a + ?Sized
{
    #[cfg(debug_assertions)]
    inner: cell::Ref<'a, T>,

    #[cfg(not(debug_assertions))]
    inner: &'a T,
}

pub struct RefMut<'a, T>
    where T: 'a + ?Sized
{
    #[cfg(debug_assertions)]
    inner: cell::RefMut<'a, T>,

    #[cfg(not(debug_assertions))]
    inner: &'a mut T,
}

impl<T> UnsafeRefCell<T>
{
    pub const fn new(value: T) -> Self
    {
        #[cfg(debug_assertions)]
        return Self{inner: cell::RefCell::new(value)};

        #[cfg(not(debug_assertions))]
        return Self{inner: cell::UnsafeCell::new(value)};
    }
}

impl<T> UnsafeRefCell<T>
    where T: ?Sized
{
    pub unsafe fn borrow(&self) -> Ref<T>
    {
        #[cfg(debug_assertions)]
        return Ref{inner: self.inner.borrow()};

        #[cfg(not(debug_assertions))]
        return Ref{inner: &*self.inner.get()};
    }

    pub unsafe fn borrow_mut(&self) -> RefMut<T>
    {
        #[cfg(debug_assertions)]
        return RefMut{inner: self.inner.borrow_mut()};

        #[cfg(not(debug_assertions))]
        return RefMut{inner: &mut *self.inner.get()};
    }
}

impl<'a, T> Deref for Ref<'a, T>
{
    type Target = T;

    fn deref(&self) -> &Self::Target
    {
        self.inner.deref()
    }
}

impl<'a, T> Deref for RefMut<'a, T>
{
    type Target = T;

    fn deref(&self) -> &Self::Target
    {
        self.inner.deref()
    }
}

impl<'a, T> DerefMut for RefMut<'a, T>
{
    fn deref_mut(&mut self) -> &mut Self::Target
    {
        self.inner.deref_mut()
    }
}
