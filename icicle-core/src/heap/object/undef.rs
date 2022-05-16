use {
    super::{super::{Heap, StackRoot}, CreateInfo, Kind, ObjectHeader, View},
    std::{mem::size_of, ptr::NonNull},
};

/// In-memory representation of undef objects.
#[repr(C)]
pub struct Undef
{
    header: ObjectHeader,
}

impl Undef
{
    pub (in super::super) unsafe fn create_info()
        -> CreateInfo<impl FnOnce(NonNull<()>)>
    {
        CreateInfo{
            size: size_of::<Self>(),
            init: |ptr| {
                let ptr = ptr.as_ptr().cast::<Self>();
                let header = ObjectHeader{kind: Kind::Undef};
                *ptr = Self{header};
            },
        }
    }

    /// Obtain the pre-allocated undef object.
    pub fn new<'h>(heap: &Heap<'h>, into: &StackRoot<'h>)
    {
        let object = heap.pre_alloc.undef();

        // SAFETY: Pre-allocated objects are always live.
        unsafe { into.set_unsafe(object) };
    }

    /// View an undef object.
    pub fn view(&self) -> View
    {
        View::Undef
    }
}
