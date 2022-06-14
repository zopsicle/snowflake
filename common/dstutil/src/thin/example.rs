# #![feature(layout_for_ptr)]
# #![feature(ptr_metadata)]
#
# use dstutil::thin::{EnableThin, ThinRef};
use std::{mem::size_of, ptr::from_raw_parts};

/// String that is prefixed with its length.
pub struct PascalStr
{
    len: usize,
    data: str,
}

unsafe impl EnableThin for PascalStr
{
    unsafe fn fatten(this: *const ()) -> *const Self
    {
        // Create a pointer with a dummy length,
        // just so that we can dereference it.
        let dummy = from_raw_parts::<Self>(this, 0);

        // Recreate the pointer but with the correct length.
        from_raw_parts(this, (*dummy).len)
    }
}

// Function for creating a PascalStr object on the heap.
fn new_pascal_str(s: &str) -> Box<PascalStr>
{
    // How to initialize custom DSTs is complicated but also irrelevant,
    // so the lengthy body of this function is hidden from the example.
#   use std::{
#       alloc::{Layout, alloc, handle_alloc_error},
#       ptr::{addr_of_mut, copy_nonoverlapping, from_raw_parts_mut, null},
#   };
#   unsafe {
#       // Compute the layout for the PascalStr object.
#       // FIXME: for_value_raw requires size to fit in isize.
#       let dummy_ptr = from_raw_parts::<PascalStr>(null(), s.len());
#       let layout = Layout::for_value_raw(dummy_ptr);
#
#       // Allocate memory for the PascalStr object.
#       let ptr = alloc(layout);
#       if ptr.is_null() {
#           handle_alloc_error(layout);
#       }
#
#       // Initialize the allocated memory.
#       let ptr = from_raw_parts_mut::<PascalStr>(ptr.cast(), s.len());
#       (*ptr).len = s.len();
#       copy_nonoverlapping(
#           /* src */ s.as_ptr(),
#           /* dst */ addr_of_mut!((*ptr).data).cast(),
#           /* len */ s.len(),
#       );
#
#       // Create a box for safe usage.
#       Box::from_raw(ptr)
#   }
}

// ThinRef<PascalStr> can be used just like &PascalStr!
let pascal_str_box = new_pascal_str("Hello, world!");
let pascal_str_thin_ref = ThinRef::new(&pascal_str_box);
assert_eq!(&pascal_str_thin_ref.data, "Hello, world!");

// A normal reference to a Pascal string is two words.
// One word stores the address, the other word stores the length.
assert_eq!(size_of::<&PascalStr>(), 2 * size_of::<usize>());

// A thin reference to a Pascal string is only one word!
// The length can already be found in the PascalStr::len field.
assert_eq!(size_of::<ThinRef<PascalStr>>(), size_of::<usize>());
