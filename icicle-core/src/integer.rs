//! Small or big integers as used in Icicle.

use gmp_sys::mp_limb_t;

/// Borrowed integer.
#[derive(Debug)]
pub enum Int<'a>
{
    /// The integer is small and stored inline.
    Small(i64),

    /// The integer is potentially big and stored elsewhere.
    ///
    /// The representation is as expected by the `mpn_*` functions of libgmp.
    /// This variant may be used even if the integer would fit into [`Small`].
    ///
    /// [`Small`]: `Self::Small`
    Big(&'a [mp_limb_t]),
}
