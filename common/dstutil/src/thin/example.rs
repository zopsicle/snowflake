# #![feature(ptr_metadata)]
#
use dstutil::{CustomDst, thin::{EnableThin, ThinRef}};
use std::{mem::{size_of, transmute}, ptr::{copy_nonoverlapping, from_raw_parts}};

/// String that is prefixed with its length.
#[repr(transparent)]
pub struct PascalStr(CustomDst<usize, str>);

unsafe impl EnableThin for PascalStr
{
    unsafe fn fatten(this: *const ()) -> *const Self
    {
        // Create a pointer with a dummy length,
        // just so that we can dereference it.
        let dummy = from_raw_parts::<Self>(this, 0);

        // Recreate the pointer but with the correct length.
        from_raw_parts(this, (*dummy).0.head)
    }
}

// Function for creating a PascalStr object on the heap.
fn new_pascal_str(s: &str) -> Box<PascalStr>
{
    unsafe {
        let inner = CustomDst::<usize, str>::new_boxed(s.len(), s.len(), |d| {
            copy_nonoverlapping::<u8>(s.as_ptr(), d.cast(), s.len());
        });
        transmute::<Box<CustomDst<usize, str>>, Box<PascalStr>>(inner)
    }
}

// ThinRef<PascalStr> can be used just like &PascalStr!
let pascal_str_box = new_pascal_str("Hello, world!");
let pascal_str_thin_ref = ThinRef::new(&pascal_str_box);
assert_eq!(&pascal_str_thin_ref.0.tail, "Hello, world!");

// A normal reference to a Pascal string is two words.
// One word stores the address, the other word stores the length.
assert_eq!(size_of::<&PascalStr>(), 2 * size_of::<usize>());

// A thin reference to a Pascal string is only one word!
// The length can already be found in the CustomDst::head field.
assert_eq!(size_of::<ThinRef<PascalStr>>(), size_of::<usize>());
