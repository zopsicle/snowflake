//! Common Snowflake action implementations.
//!
#![doc = snowflake_util::see_manual!()]

#![doc(html_logo_url = "/snowflake-manual/_static/logo.svg")]
#![feature(assert_matches)]
#![feature(concat_bytes)]
#![feature(exit_status_error)]
#![feature(io_safety)]
#![feature(let_else)]
#![feature(panic_always_abort)]
#![feature(type_ascription)]
#![warn(missing_docs)]

pub use self::{create_symbolic_link::*, run_command::*, write_regular_file::*};

mod create_symbolic_link;
mod run_command;
mod write_regular_file;
