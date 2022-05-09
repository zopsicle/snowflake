use {super::{Val, Value, off_heap_tag}, std::hint::unreachable_unchecked};

const FALSE_BITS: u64 = 0 << 4 | off_heap_tag::BOOLEAN;
const TRUE_BITS:  u64 = 1 << 4 | off_heap_tag::BOOLEAN;

/// Working with Boolean values.
impl Value
{
    /// Create a Boolean value.
    pub fn boolean_from_bool(value: bool) -> Self
    {
        let off_heap = if value { TRUE_BITS } else { FALSE_BITS };
        unsafe { Self::from_off_heap(off_heap) }
    }

    /// See [`Self::borrow_off_heap`].
    pub (super) unsafe fn borrow_off_heap_boolean(&self) -> Val
    {
        match self.inner.get() {
            FALSE_BITS => Val::Boolean(false),
            TRUE_BITS  => Val::Boolean(true),
            _ => unreachable_unchecked(),
        }
    }
}

#[cfg(test)]
mod tests
{
    use {super::*, std::assert_matches::assert_matches};

    #[test]
    fn roundtrip()
    {
        let value = Value::boolean_from_bool(false);
        assert_matches!(value.borrow(), Val::Boolean(false));

        let value = Value::boolean_from_bool(true);
        assert_matches!(value.borrow(), Val::Boolean(true));
    }
}
