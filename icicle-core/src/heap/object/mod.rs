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

#[cfg(test)]
mod tests
{
    use {
        crate::istring::IString,
        super::{*, super::{BorrowRef, Heap, PinnedRef, StackRoot}},
        proptest::{self as p, proptest, strategy::Strategy},
    };

    /// Owned counterpart to [`View`].
    #[derive(Debug)]
    enum Template
    {
        Undef,
        Boolean(bool),
        String(IString),
    }

    impl Template
    {
        /// Generate arbitrary templates.
        fn any() -> impl Strategy<Value=Template>
        {
            p::prop_oneof![
                1  => p::strategy::LazyJust::new(|| Self::Undef),
                5  => p::arbitrary::any::<bool>()
                    .prop_map(Self::Boolean),
                10 => p::arbitrary::any::<Vec<u8>>()
                    .prop_map(IString::from_bytes)
                    .prop_map(Self::String),
            ]
        }

        /// Create a new object from the template.
        fn new<'h>(&self, heap: &'h Heap<'h>, root: &StackRoot<'h>)
        {
            match self {
                Self::Undef =>
                    Undef::new(heap, root),
                Self::Boolean(value) =>
                    Boolean::new_from_bool(heap, root, *value),
                Self::String(bytes) =>
                    String::new_from_bytes(heap, root, bytes.as_bytes()),
            }
        }

        /// Assert that an object matches the template.
        fn assert<'h>(&self, object: &impl PinnedRef<'h>)
        {
            let view = object.view();
            let ok = match (self, view) {
                (Self::Undef,       View::Undef      ) => true,
                (Self::Boolean(v1), View::Boolean(v2)) => *v1 == v2,
                (Self::String(b1),  View::String(b2) ) => b1 == b2,
                _ => false,
            };
            assert!(ok, "assertion failed:\n\
                         template: `{self:?}`,\n    \
                         view: `{view:?}`");
        }
    }

    proptest!
    {
        /// Allocate a bunch of objects and test that they roundtrip.
        #[test]
        fn roundtrip(
            templates in p::collection::vec(
                Template::any(),
                0 ..= 100,
            ),
        )
        {
            Heap::with(|heap| {
                let mut cases = Vec::new();
                heap.with_stack_roots(|[root]| {
                    for template in templates {
                        template.new(heap, root);
                        cases.push((template, root.pin()));
                    }
                });
                for (template, root) in cases {
                    template.assert(&root);
                }
            });
        }
    }
}
