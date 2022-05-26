//! Snowflake build system.

#![doc(html_logo_url = "/snowflake-manual/_static/logo.svg")]
#![feature(io_safety)]
#![feature(let_chains)]
#![feature(once_cell)]
#![warn(missing_docs)]

pub mod action;
pub mod basename;
pub mod hash;
pub mod label;
pub mod state;

mod util;
