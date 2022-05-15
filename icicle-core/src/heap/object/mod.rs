//! Working with objects on garbage-collected heaps.

pub use self::{boolean::*, string::*, undef::*, view::*};

use std::{mem::align_of, ptr::NonNull};

mod boolean;
mod string;
mod undef;
mod view;

/// Ensure that what embeds this is at least object-aligned.
#[repr(align(8))]
pub struct ObjectAlign;

/// Minimum required alignment for objects.
pub const OBJECT_ALIGN: usize = align_of::<ObjectAlign>();

/// Information on how to create an object.
pub (super) struct CreateInfo<F>
    where F: FnOnce(NonNull<()>)
{
    /// How many bytes to allocate for the object.
    pub size: usize,

    /// Function that initializes the object.
    pub init: F,
}

/// Data at the start of each object.
///
/// Every object representation type must begin with a field of this type.
/// And they must use `#[repr(C)]` so that we can downcast from this type.
pub struct ObjectHeader
{
    /// What kind of object this is.
    pub kind: Kind,
}

/// Kind of object.
///
/// This tells you which of the different Rust representation types is used.
/// For example, if [`ObjectHeader::kind`] is set to [`Kind::Boolean`],
/// then the object is represented by the [`Boolean`] struct.
#[allow(missing_docs)]
#[derive(Clone, Copy)]
pub enum Kind
{
    Undef,
    Boolean,
    String,
}
