//! Implementation details of the heap system.
//!
//! # Design of the heap system
//!
//! This section explains the important concepts within the heap system.
//! The documentation on the individual items is intentionally left sparse;
//! refer to this section for all the important design information.
//!
//! ## Garbage-collected heaps
//!
//! A [garbage-collected heap][`GcHeap`] implements
//! tracing garbage collection for freeing memory.
//!
//! Objects that live in garbage-collected heaps
//! may not be referenced from other heaps.
//!
//! ## Compact regions
//!
//! A [compact region][`CompactRegion`] is a type of heap
//! that has the following properties:
//!
//!  - Objects in a compact region are not subject to garbage collection.
//!  - Objects in a compact region cannot mutably point to other objects.
//!  - Objects in a compact region only point to objects in compact regions.
//!
//! Objects that live in compact regions are said to be _compacted_.
//! Compacted objects may be referenced from other heaps.
//! This allows for efficient sharing of large amounts of data between fibers.
//! The downside is that compact regions are only destroyed as a whole;
//! compacted objects cannot be individually destroyed.
//!
//! Compact regions themselves are atomically reference counted;
//! they are automatically destroyed when nobody references them anymore.
//! Shared ownership of compact regions exists in a few places, most notably:
//!
//!  - A garbage-collected heap has shared ownership of any compact region
//!    it possibly contains references to compacted objects from.
//!    The shared ownership is recomputed on each garbage collection cycle,
//!    and also when compacted objects are received from channels.
//!
//!  - A compact region has shared ownership of any compact region
//!    it possibly contains references to compacted objects from,
//!    except for references to compacted objects from itself.
//!    The shared ownership is recomputed when objects containing
//!    references to compacted objects are added to the compact region.
//!    How to deal with cyclic references between compact regions
//!    is currently an unsolved problem; you'll get memory leaks.
//!
//!  - Every queue element of a channel tracks
//!    which compact regions its objects live in.
//!    (During object serialization when sending over a channel,
//!    compacted objects are serialized as their address,
//!    along with an arc to their owning compact region.)
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

pub use self::{block::*, compact_region::*, gc_heap::*};

mod block;
mod compact_region;
mod gc_heap;
