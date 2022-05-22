#![feature(extern_types)]
#![feature(once_cell)]
#![warn(missing_docs)]

use self::backend::Backend;

pub mod ir;
pub mod syntax;

mod backend;

/// Sekka virtual machine with its own garbage-collected heap.
pub struct Isolate
{
    backend: Backend,
}

impl Isolate
{
    /// Create a new isolate.
    pub fn new() -> Self
    {
        Self{backend: Backend::new()}
    }
}
