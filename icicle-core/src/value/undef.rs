use super::{Val, Value, off_heap_tag};

const BITS: u64 = off_heap_tag::UNDEF;

/// Working with undef values.
impl Value
{
    /// Create an undef value.
    pub fn undef() -> Self
    {
        unsafe { Self::from_off_heap(BITS) }
    }

    /// See [`Self::borrow_off_heap`].
    pub (super) unsafe fn borrow_off_heap_undef(&self) -> Val
    {
        Val::Undef
    }
}

#[cfg(test)]
mod tests
{
    use {super::*, std::assert_matches::assert_matches};

    #[test]
    fn roundtrip()
    {
        let value = Value::undef();
        assert_matches!(value.borrow(), Val::Undef);
    }
}
