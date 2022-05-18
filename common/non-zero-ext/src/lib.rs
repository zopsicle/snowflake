//! Extra methods for non-zero integers.

#![warn(missing_docs)]

use std::num::NonZeroU64;

/// Extra methods for non-zero integers.
pub trait NonZeroExt
{
    /// The number 1.
    const ONE: Self;
}

impl NonZeroExt for NonZeroU64
{
    const ONE: Self = unsafe { Self::new_unchecked(1) };
}
