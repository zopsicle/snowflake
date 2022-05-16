//! Implementation details of the heap system.
//!
//! # Design of the heap system
//!
//! This section explains the important concepts within the heap system.
//! The documentation on the individual items is intentionally left sparse;
//! refer to this section for all the important design information.
//!
//! ## Blocks
//!
//! A [block][`Block`] is a region of memory in which objects are located.
//! Blocks have two important properties:
//! each block is aligned to [`BLOCK_ALIGN`] bytes;
//! and each object in a block is located in
//! the first [`BLOCK_ALIGN`] bytes of the block.
//! These properties enable the discovery of a block's address
//! given just the address of any of the objects in the block,
//! by rounding down the address of any value
//! to the nearest multiple of [`BLOCK_ALIGN`].
//!
//! Each block begins with a [block header][`BlockHeader`],
//! which contains information about the owner of the block.
//! The objects in the block immediately follow the block header.
//! Allocation proceeds by bumping a pointer until the block is full.
//!
//! Padding bytes may exist between adjacent objects in a block.
//! But no more than the minimum required for alignment,
//! so the garbage collector can traverse the block.

pub use self::block::*;

use std::marker::PhantomPinned;

mod block;

/// Garbage-collected heap.
pub struct GcHeap
{
    /// Each block stores a pointer to the heap.
    _pinned: PhantomPinned,
}

/// Compact region.
pub struct CnfHeap
{
    /// Each block stores a pointer to the heap.
    _pinned: PhantomPinned,
}
