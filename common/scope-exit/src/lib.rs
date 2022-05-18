//! Ad-hoc scope guards.

#![warn(missing_docs)]

use std::mem::ManuallyDrop;

/// Non-macro form of [`scope_exit`].
///
/// This is useful if you want to [`forget`] the scope guard at some point,
/// or otherwise use the scope guard as an object.
///
/// [`forget`]: `std::mem::forget`.
pub struct ScopeExit<F>
    where F: FnOnce()
{
    f: ManuallyDrop<F>,
}

impl<F> ScopeExit<F>
    where F: FnOnce()
{
    /// Create a scope guard that calls a given function on drop.
    pub fn new(f: F) -> Self
    {
        Self{f: ManuallyDrop::new(f)}
    }
}

impl<F> Drop for ScopeExit<F>
    where F: FnOnce()
{
    fn drop(&mut self)
    {
        // SAFETY: self.f will not be used anymore.
        let f = unsafe { ManuallyDrop::take(&mut self.f) };
        f();
    }
}

/// Define an ad-hoc scope guard.
///
/// The code passed to this macro is performed at the end of the scope.
/// It is performed when the scope ends or when a panic passes through.
///
/// # Examples
///
/// ```
/// # use scope_exit::scope_exit;
/// use std::cell::Cell;
/// let x = Cell::new(0);
/// {
///     scope_exit! { x.set(1); }
///     x.set(2);
/// }
/// assert_eq!(x.get(), 1);
/// ```
#[macro_export]
macro_rules! scope_exit
{
    { $($tt:tt)* } => {
        let __scope_exit = $crate::ScopeExit::new(|| { $($tt)* });
    };
}
