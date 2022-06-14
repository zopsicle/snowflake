//! Thin pointers to dynamically sized types.
//!
//! # Examples
//!
//! ```
#![doc = include_str!("example.rs")]
//! ```

pub use self::{thin_non_null::*, thin_ref::*};

mod thin_non_null;
mod thin_ref;

/// Trait for obtaining a fat pointer from a thin pointer.
///
/// This trait can be implemented for types whose values
/// contain enough information to reconstruct pointer metadata.
/// For example, an array which stores its length inside itself.
///
/// # Safety
///
/// The pointer part of the fat pointer must equal `this`.
/// The returned pointer must be safe to dereference.
pub unsafe trait EnableThin
{
    /// Obtain a fat pointer from a thin pointer.
    ///
    /// # Safety
    ///
    /// Ignoring its metadata, `this` must be safe to dereference.
    unsafe fn fatten(this: *const ()) -> *const Self;

    /// Obtain a fat pointer from a thin pointer.
    ///
    /// The default implementation calls [`fatten`][`Self::fatten`].
    ///
    /// # Safety
    ///
    /// Ignoring its metadata, `this` must be safe to dereference.
    unsafe fn fatten_mut(this: *mut ()) -> *mut Self
    {
        Self::fatten(this) as *mut Self
    }
}

/// Thin pointers are always available for sized types.
unsafe impl<T> EnableThin for T
{
    unsafe fn fatten(this: *const ()) -> *const Self
    {
        this.cast()
    }
}
