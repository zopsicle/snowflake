//! Parser and interpreter for Icicle.

#![feature(allocator_api)]
#![feature(assert_matches)]
#![feature(box_into_pin)]
#![feature(default_free_fn)]
#![feature(get_mut_unchecked)]
#![feature(int_roundings)]
#![feature(maybe_uninit_write_slice)]
#![feature(never_type)]
#![feature(new_uninit)]
#![warn(missing_docs)]

pub mod bytecode;
pub mod heap;
pub mod integer;
pub mod istring;
pub mod syntax;
