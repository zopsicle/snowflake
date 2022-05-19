//! In-memory representations of objects.
//!
//! For each type of object, there is a Rust type in this module,
//! along with functions to compute the number of bytes to allocate
//! and to initialize the memory for the object after allocation.
//! Each of the initialization functions returns an [`ObjectRef`]
//! that is equivalent to the provided pointer, for convenience.
//! These methods are not documented; see their wrappers
//! in [`Fiber`][`super::Fiber`] and [`CompactRegion`].

#![allow(missing_docs)]

use {
    crate::bytecode,
    super::CompactRegion,
    std::{
        cell::Cell,
        fmt,
        mem::{MaybeUninit, size_of, size_of_val},
        ptr::{self, NonNull},
        slice,
    },
};

/// Minimum alignment for objects.
pub const OBJECT_ALIGN: usize = 8;

/// Reference to an object.
#[derive(Clone, Copy)]
pub struct ObjectRef
{
    pub ptr: NonNull<ObjectHeader>,
}

impl ObjectRef
{
    /// Create a dangling object reference
    pub fn dangling() -> Self
    {
        Self{ptr: NonNull::dangling()}
    }
}

impl fmt::Debug for ObjectRef
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        fmt::Debug::fmt(&self.ptr, f)
    }
}

/// Type of object.
pub enum ObjectHeader
{
    Undef,
    Boolean,
    String,
    Array,
    Slot,
    Procedure,
    CompactRegionHandle,
}

/* -------------------------------------------------------------------------- */
/*                                    Undef                                   */
/* -------------------------------------------------------------------------- */

#[repr(C)]
pub struct Undef
{
    pub header: ObjectHeader,
}

pub fn undef_size() -> usize
{
    size_of::<Undef>()
}

pub unsafe fn undef_init(ptr: *mut ()) -> ObjectRef
{
    let ptr = ptr.cast::<Undef>();
    let header = ObjectHeader::Undef;
    ptr::write(ptr, Undef{header});
    ObjectRef{ptr: NonNull::new_unchecked(ptr).cast()}
}

/* -------------------------------------------------------------------------- */
/*                                   Boolean                                  */
/* -------------------------------------------------------------------------- */

#[repr(C)]
pub struct Boolean
{
    pub header: ObjectHeader,
    pub value: bool,
}

pub fn boolean_size() -> usize
{
    size_of::<Boolean>()
}

pub unsafe fn boolean_init_from_bool(ptr: *mut (), value: bool) -> ObjectRef
{
    let ptr = ptr.cast::<Boolean>();
    let header = ObjectHeader::Boolean;
    ptr::write(ptr, Boolean{header, value});
    ObjectRef{ptr: NonNull::new_unchecked(ptr).cast()}
}

/* -------------------------------------------------------------------------- */
/*                                   String                                   */
/* -------------------------------------------------------------------------- */

#[repr(C)]
pub struct String
{
    pub header: ObjectHeader,
    pub len: usize,
    pub bytes: [u8; 0 /* len + 1 */],
}

pub fn string_size(len: usize) -> usize
{
    // TODO: Handle integer overflow.
    size_of::<String>() + len + 1
}

pub unsafe fn string_init_from_fn<F>(ptr: *mut (), len: usize, f: F)
    -> ObjectRef
    where F: FnOnce(&mut [MaybeUninit<u8>])
{
    let ptr = ptr.cast::<String>();
    let header = ObjectHeader::String;
    ptr::write(ptr, String{header, len, bytes: []});
    f(slice::from_raw_parts_mut((*ptr).bytes.as_mut_ptr().cast(), len));
    *(*ptr).bytes.get_unchecked_mut(len) = 0;
    ObjectRef{ptr: NonNull::new_unchecked(ptr).cast()}
}

/* -------------------------------------------------------------------------- */
/*                                    Array                                   */
/* -------------------------------------------------------------------------- */

#[repr(C)]
pub struct Array
{
    pub header: ObjectHeader,
    pub len: usize,
    pub elements: [ObjectRef; 0 /* len */],
}

pub fn array_size(len: usize) -> usize
{
    // TODO: Handle integer overflow.
    size_of::<Array>() + len * size_of::<ObjectRef>()
}

pub unsafe fn array_init_from_fn<F>(ptr: *mut (), len: usize, f: F)
    -> ObjectRef
    where F: FnOnce(&mut [MaybeUninit<ObjectRef>])
{
    let ptr = ptr.cast::<Array>();
    let header = ObjectHeader::Array;
    ptr::write(ptr, Array{header, len, elements: []});
    f(slice::from_raw_parts_mut((*ptr).elements.as_mut_ptr().cast(), len));
    ObjectRef{ptr: NonNull::new_unchecked(ptr).cast()}
}

/* -------------------------------------------------------------------------- */
/*                                    Slot                                    */
/* -------------------------------------------------------------------------- */

#[repr(C)]
pub struct Slot
{
    pub header: ObjectHeader,

    // Slots cannot appear in compact regions, only in fibers.
    // So we don't need to synchronize their access.
    pub cell: Cell<ObjectRef>,
}

pub fn slot_size() -> usize
{
    size_of::<Slot>()
}

pub unsafe fn slot_init_from_object_ref(ptr: *mut (), object_ref: ObjectRef)
    -> ObjectRef
{
    let ptr = ptr.cast::<Slot>();
    let header = ObjectHeader::Slot;
    let cell = Cell::new(object_ref);
    ptr::write(ptr, Slot{header, cell});
    ObjectRef{ptr: NonNull::new_unchecked(ptr).cast()}
}

/* -------------------------------------------------------------------------- */
/*                                  Procedure                                 */
/* -------------------------------------------------------------------------- */

#[repr(C)]
pub struct Procedure
{
    pub header: ObjectHeader,
    pub max_register: Option<bytecode::Register>,
    pub len: usize,
    pub instructions: [bytecode::Instruction; 0 /* len */],
}

pub fn procedure_size(procedure: &bytecode::verify::Verified) -> usize
{
    // TODO: Handle integer overflow.
    size_of::<Procedure>() + size_of_val::<[_]>(&procedure.instructions)
}

pub unsafe fn procedure_init_from_verified(
    ptr: *mut (),
    procedure: &bytecode::verify::Verified,
) -> ObjectRef
{
    let ptr = ptr.cast::<Procedure>();
    let header = ObjectHeader::Procedure;
    let max_register = procedure.max_register;
    let len = procedure.instructions.len();
    ptr::write(ptr, Procedure{header, max_register, len, instructions: []});
    slice::from_raw_parts_mut((*ptr).instructions.as_mut_ptr(), len)
        .copy_from_slice(&procedure.instructions);
    ObjectRef{ptr: NonNull::new_unchecked(ptr).cast()}
}

/* -------------------------------------------------------------------------- */
/*                             CompactRegionHandle                            */
/* -------------------------------------------------------------------------- */

#[repr(C)]
pub struct CompactRegionHandle
{
    pub header: ObjectHeader,
    pub compact_region: *const CompactRegion,
}

pub fn compact_region_handle_size() -> usize
{
    size_of::<CompactRegionHandle>()
}

pub unsafe fn compact_region_handle_init_from_compact_region(
    ptr: *mut (),
    compact_region: *const CompactRegion,
) -> ObjectRef
{
    let ptr = ptr.cast::<CompactRegionHandle>();
    let header = ObjectHeader::CompactRegionHandle;
    ptr::write(ptr, CompactRegionHandle{header, compact_region});
    ObjectRef{ptr: NonNull::new_unchecked(ptr).cast()}
}
