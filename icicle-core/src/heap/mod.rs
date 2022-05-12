//! Garbage-collected heaps.
//!
//! Garbage-collected heaps can be implemented entirely in safe Rust,
//! with a layer of indirection using abstract "object identifiers".
//! However, this implementation uses pointers to objects directly.
//! With the desire to keep the interface safe, this is non-trivial.
//! The interface hence provides various ways to
//! interact with the heap and heap-allocated objects,
//! with different ways to keep track of root references.
//! Accessing objects through root references is completely safe,
//! and by using the right types of root references, has little overhead.
//!
//! # Components
//!
//! The diagram below shows the relationships between
//! the different types of objects in this module.
//!
#![doc = include_str!(concat!(env!("OUT_DIR"), "/heap_diagram.drawio.svg"))]
//!
//! ## Notes
//!
//!  1. The object borrows using a pointer or reference.
//!  2. The object borrows using pointer arithmetic on itself.
//!  3. The object registers and unregisters itself
//!     during construction, cloning, and dropping.

pub use self::{
    block::*,
    borrow_ref::*,
    heap::*,
    mutator::*,
    object::*,
    pinned_root::*,
    pre_alloc::*,
    unsafe_ref::*,
};

mod block;
mod borrow_ref;
mod heap;
mod mutator;
mod object;
mod pinned_root;
mod pre_alloc;
mod unsafe_ref;
