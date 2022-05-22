//! Locations within source code.

use std::fmt;

/// A source location within a source file.
#[derive(Clone, Copy)]
pub struct Location
{
    /// The byte offset in the source file.
    pub offset: usize,
}

impl fmt::Debug for Location
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        // We explicitly *do not* want to use f.debug_tuple,
        // as that would insert noisy newlines with {:#?}.
        write!(f, "Location({:?})", self.offset)
    }
}
