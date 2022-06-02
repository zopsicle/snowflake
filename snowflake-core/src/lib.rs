//! Snowflake action and caching infrastructure.

#![feature(assert_matches)]
#![feature(concat_bytes)]
#![feature(exit_status_error)]
#![feature(io_error_other)]
#![feature(io_safety)]
#![feature(once_cell)]
#![warn(missing_docs)]

pub mod action;
pub mod label;
pub mod state;
