//! Utilities for working with custom dynamically-sized types.

#![feature(allocator_api)]
#![feature(layout_for_ptr)]
#![feature(ptr_metadata)]
#![warn(missing_docs)]

use {
    scope_exit::ScopeExit,
    std::{
        alloc::{AllocError, Allocator, Global, Layout},
        mem::forget,
        ptr::{Pointee, addr_of_mut, from_raw_parts, from_raw_parts_mut, null},
    },
};

pub mod thin;

/// Custom dynamically-sized type.
///
/// Consists of a sized head `H` and a dynamically-sized tail `T`.
/// This type is `repr(C)`, ensuring that the offset of `head` is zero.
#[allow(missing_docs)]
#[repr(C)]
pub struct CustomDst<H, T>
    where T: ?Sized
{
    pub head: H,
    pub tail: T,
}

impl<H, T> CustomDst<H, T>
    where T: ?Sized
{
    /// Allocate and initialize a custom dynamically-sized value on the heap.
    ///
    /// The `tail_metadata` argument becomes the metadata for the fat pointer.
    /// The `tail_init` function initializes the tail of the value.
    /// If `tail_init` panics, the tail is not dropped.
    ///
    /// # Safety
    ///
    /// `tail_metadata` must be suitable for the value
    /// that `tail_init` is going to initialize.
    /// When `tail_init` returns, the tail must be initialized.
    pub unsafe fn new_boxed(
        head: H,
        tail_metadata: <Self as Pointee>::Metadata,
        tail_init: impl FnOnce(*mut T),
    ) -> Box<Self>
    {
        Self::new_boxed_in(head, tail_metadata, tail_init, Global)
    }

    /// See [`new_boxed`][`Self::new_boxed`].
    pub unsafe fn new_boxed_in<A>(
        head: H,
        metadata: <Self as Pointee>::Metadata,
        tail_init: impl FnOnce(*mut T),
        alloc: A,
    ) -> Box<Self, A>
        where A: Allocator
    {
        match Self::try_new_boxed_in(head, metadata, tail_init, alloc) {
            Ok(boxed) => boxed,
            Err(_) => todo!("Call handle_alloc_error with correct layout"),
        }
    }

    /// See [`new_boxed`][`Self::new_boxed`].
    pub unsafe fn try_new_boxed(
        head: H,
        tail_metadata: <Self as Pointee>::Metadata,
        tail_init: impl FnOnce(*mut T),
    ) -> Result<Box<Self>, AllocError>
    {
        Self::try_new_boxed_in(head, tail_metadata, tail_init, Global)
    }

    /// See [`new_boxed`][`Self::new_boxed`].
    pub unsafe fn try_new_boxed_in<A>(
        head: H,
        metadata: <Self as Pointee>::Metadata,
        tail_init: impl FnOnce(*mut T),
        alloc: A,
    ) -> Result<Box<Self, A>, AllocError>
        where A: Allocator
    {
        // Compute the layout for the dynamically-sized value.
        let dummy_ptr = from_raw_parts::<Self>(null(), metadata);
        // FIXME: This is currently unsafe because Layout::for_value_raw
        //        requires the size to fit in isize and we don't check that.
        let layout = Layout::for_value_raw(dummy_ptr);

        // Allocate memory for the dynamically-sized value.
        let ptr = alloc.allocate(layout)?.cast::<u8>();
        let fat = from_raw_parts_mut::<Self>(ptr.as_ptr().cast(), metadata);

        // Initialize the tail first, so that head is dropped on panic.
        let init_guard = ScopeExit::new(|| alloc.deallocate(ptr, layout));
        tail_init(addr_of_mut!((*fat).tail));
        forget(init_guard);

        // Initialize the head.
        (*fat).head = head;

        // Create the box to be returned.
        Ok(Box::from_raw_in(fat, alloc))
    }
}
