//! Sekka programming language.

#![doc(html_logo_url = "/sekka-manual/_static/logo.svg")]
#![feature(get_mut_unchecked)]
#![feature(maybe_uninit_write_slice)]
#![feature(new_uninit)]
#![warn(missing_docs)]

pub mod bytecode;
pub mod compile;
pub mod interpret;
pub mod syntax;
pub mod value;

mod util;
