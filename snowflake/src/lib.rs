//! Snowflake build system.

#![feature(io_safety)]
#![feature(let_chains)]
#![feature(once_cell)]
#![warn(missing_docs)]

pub mod action;
pub mod basename;
pub mod hash;
pub mod label;
pub mod state;
