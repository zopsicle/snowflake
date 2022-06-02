//! Snowflake build system.
//!
//! The documentation for this crate does not define all terms.
//! Refer to the [Snowflake manual] for a thorough description
//! of all the terms and their concepts involved.
//! Especially the [index] might be of interest.
//!
//! [Snowflake manual]: /snowflake-manual/index.html
//! [index]: /snowflake-manual/genindex.html

#![doc(html_logo_url = "/snowflake-manual/_static/logo.svg")]
#![feature(assert_matches)]
#![feature(concat_bytes)]
#![feature(const_btree_new)]
#![feature(exit_status_error)]
#![feature(io_error_other)]
#![feature(io_safety)]
#![feature(let_chains)]
#![feature(let_else)]
#![feature(once_cell)]
#![feature(panic_always_abort)]
#![feature(type_ascription)]
#![warn(missing_docs)]

pub mod action;
pub mod label;
pub mod state;
