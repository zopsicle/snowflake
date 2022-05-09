//! Parser and interpreter for Icicle.

#![feature(assert_matches)]
#![feature(let_else)]
#![feature(maybe_uninit_array_assume_init)]
#![feature(maybe_uninit_uninit_array)]
#![feature(maybe_uninit_write_slice)]
#![warn(missing_docs)]

pub mod bytecode;
pub mod integer;
pub mod istring;
pub mod syntax;
pub mod value;
