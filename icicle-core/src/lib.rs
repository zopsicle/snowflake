//! Parser and interpreter for Icicle.

#![feature(allocator_api)]
#![feature(assert_matches)]
#![feature(default_free_fn)]
#![feature(int_roundings)]
#![warn(missing_docs)]

pub mod bytecode;
pub mod heap;
pub mod integer;
pub mod istring;
pub mod syntax;
