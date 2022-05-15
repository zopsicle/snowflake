use {crate::istring::IStr, super::{super::PinnedRef, ObjectHeader, Kind}};

/// Borrowed view into an object.
#[allow(missing_docs)]
#[derive(Clone, Copy, Debug)]
pub enum View<'a>
{
    Undef,
    Boolean(bool),
    String(&'a IStr),
}

impl<'a> View<'a>
{
    /// Borrow the contents of an object.
    pub fn of<'h, R>(object: &R) -> Self
        where R: PinnedRef<'h> + ?Sized
    {
        let ptr = object.borrow_ref().as_ptr().as_ptr();
        let ptr = ptr.cast::<ObjectHeader>();

        // SAFETY: The object is live, as guaranteed by PinnedRef.
        let kind = unsafe { (*ptr).kind };

        // SAFETY: The Rust type corresponds to the kind field,
        //         so these pointer dereferences are correct.
        match kind {
            Kind::Undef   => unsafe { (*ptr.cast::<super::Undef  >()).view() },
            Kind::Boolean => unsafe { (*ptr.cast::<super::Boolean>()).view() },
            Kind::String  => unsafe { (*ptr.cast::<super::String >()).view() },
        }
    }
}
