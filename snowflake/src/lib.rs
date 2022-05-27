//! Snowflake build system.

#![doc(html_logo_url = "/snowflake-manual/_static/logo.svg")]
#![feature(assert_matches)]
#![feature(exit_status_error)]
#![feature(io_error_other)]
#![feature(io_safety)]
#![feature(let_chains)]
#![feature(once_cell)]
#![feature(panic_always_abort)]
#![feature(type_ascription)]
#![warn(missing_docs)]

pub mod action;
pub mod basename;
pub mod hash;
pub mod label;
pub mod state;
