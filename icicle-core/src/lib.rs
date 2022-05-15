//! Parser and interpreter for Icicle.

#![feature(assert_matches)]
#![feature(box_into_pin)]
#![feature(dropck_eyepatch)]
#![feature(int_roundings)]
#![feature(let_else)]
#![feature(maybe_uninit_write_slice)]
#![feature(nonzero_ops)]
#![feature(strict_provenance)]
#![warn(missing_docs)]

pub mod bytecode;
pub mod heap;
pub mod integer;
pub mod istring;
pub mod syntax;
