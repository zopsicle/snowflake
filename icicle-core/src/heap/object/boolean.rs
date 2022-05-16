use {
    super::{super::{Mutator, StackRoot}, CreateInfo, Kind, ObjectHeader, View},
    std::{mem::size_of, ptr::NonNull},
};

/// In-memory representation of Boolean objects.
#[repr(C)]
pub struct Boolean
{
    header: ObjectHeader,
    value: bool,
}

impl Boolean
{
    pub (in super::super) unsafe fn create_info_from_bool(value: bool)
        -> CreateInfo<impl FnOnce(NonNull<()>)>
    {
        CreateInfo{
            size: size_of::<Self>(),
            init: move |ptr| {
                let ptr = ptr.as_ptr().cast::<Self>();
                let header = ObjectHeader{kind: Kind::Boolean};
                *ptr = Self{header, value};
            },
        }
    }

    /// Obtain a pre-allocated Boolean object.
    pub fn new_from_bool<'h>(
        mutator: &Mutator<'h>,
        into: &StackRoot<'h>,
        value: bool,
    )
    {
        let object = if value {
            mutator.heap.pre_alloc.boolean_true()
        } else {
            mutator.heap.pre_alloc.boolean_false()
        };

        // SAFETY: Pre-allocated objects are always live.
        unsafe { into.set_unsafe(object) };
    }

    /// View a Boolean object.
    pub fn view(&self) -> View
    {
        View::Boolean(self.value)
    }
}
