//! Extra items for working with allocators.

#![feature(allocator_api)]
#![warn(missing_docs)]

use std::{alloc::{AllocError, Allocator, Layout}, ptr::NonNull};

/// Ensure a minimum alignment of `ALIGN` for each allocation.
#[derive(Clone, Copy, Default)]
pub struct AligningAllocator<T, const ALIGN: usize>(pub T);

unsafe impl<T, const ALIGN: usize> Allocator for AligningAllocator<T, ALIGN>
    where T: Allocator
{
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError>
    {
        let layout = layout.align_to(ALIGN).map_err(|_| AllocError)?;
        self.0.allocate(layout)
    }

    unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout)
    {
        let layout = layout.align_to(ALIGN).unwrap_unchecked();
        self.0.deallocate(ptr, layout)
    }
}
