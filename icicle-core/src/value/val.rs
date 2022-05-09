use {
    crate::{integer::Int, istring::IStr},
    super::{OnHeapHeader, Value, off_heap_tag, on_heap_tag},
    std::hint::unreachable_unchecked,
};

/// Borrowed value.
#[allow(missing_docs)]
#[derive(Debug)]
pub enum Val<'a>
{
    Undef,
    Boolean(bool),
    Integer(Int<'a>),
    String(&'a IStr),
}

/// Borrowing values.
impl Value
{
    /// Borrow the value.
    ///
    /// This gives convenient access the value in the form of an enum.
    /// You can pattern match on it to find out what type of value this is.
    pub fn borrow(&self) -> Val
    {
        if let Some(on_heap) = self.get_on_heap() {
            Self::borrow_on_heap(on_heap)
        } else {
            self.borrow_off_heap()
        }
    }

    /// Call the correct `borrow_off_heap_*` method.
    ///
    /// # Safety
    ///
    /// Each `borrow_off_heap_*` method assumes that the value
    /// is off-heap and has a tag that it can handle.
    fn borrow_off_heap(&self) -> Val
    {
        let tag = self.inner.get() & 0b1111;
        unsafe {
            match tag {
                off_heap_tag::UNDEF   => self.borrow_off_heap_undef(),
                off_heap_tag::BOOLEAN => self.borrow_off_heap_boolean(),
                off_heap_tag::INTEGER => self.borrow_off_heap_integer(),
                off_heap_tag::STRING  => self.borrow_off_heap_string(),
                _                     => unreachable_unchecked(),
            }
        }
    }

    /// Call the correct `borrow_on_heap_*` method.
    ///
    /// # Safety
    ///
    /// Each `borrow_on_heap_*` method assumes that the value
    /// is on-heap and has a tag that it can handle.
    fn borrow_on_heap(on_heap: &OnHeapHeader) -> Val
    {
        let tag = on_heap.extra_word & 0b1111;
        unsafe {
            match tag {
                on_heap_tag::STRING => Self::borrow_on_heap_string(on_heap),
                _                   => unreachable_unchecked(),
            }
        }
    }
}
