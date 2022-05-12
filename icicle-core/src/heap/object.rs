use std::mem::align_of;

/// Ensure that what embeds this is at least object-aligned.
#[repr(align(8))]
pub struct ObjectAlign;

/// Minimum required alignment for objects.
pub const OBJECT_ALIGN: usize = align_of::<ObjectAlign>();
