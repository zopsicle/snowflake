//! Permanently unstable implementation details.
//!
//! This module provides access to objects that compose bytecode and values.
//! The current implementation relies on atomic reference counting,
//! which will be replaced with tracing garbage collection in the future.
//! When that happens the design will change completely.
//! Crate users should use the public API instead.

pub mod bytecode;
pub mod compile;
pub mod interpret;
pub mod value;
