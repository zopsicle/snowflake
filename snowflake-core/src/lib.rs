//! Snowflake action and caching infrastructure.
//!
#![doc = snowflake_util::see_manual!()]

#![doc(html_logo_url = "/snowflake-manual/_static/logo.svg")]
#![feature(assert_matches)]
#![feature(concat_bytes)]
#![feature(exit_status_error)]
#![feature(io_error_other)]
#![feature(io_safety)]
#![feature(once_cell)]
#![feature(type_ascription)]
#![warn(missing_docs)]

pub mod action;
pub mod label;
pub mod state;
