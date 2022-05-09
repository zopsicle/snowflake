use {crate::integer::Int, super::{Val, Value, off_heap_tag}};

macro_rules! integer_from_small
{
    { $($name:ident $type:ty;)* } => {
        $(
            /// Create an integer value.
            pub fn $name(value: $type) -> Self
            {
                let payload = (value as u64) << 4;
                unsafe { Self::from_off_heap(payload | off_heap_tag::INTEGER) }
            }
        )*
    };
}

/// Working with integer values.
impl Value
{
    integer_from_small! {
        integer_from_i8  i8  ;
        integer_from_i16 i16 ;
        integer_from_i32 i32 ;
        integer_from_u8  u8  ;
        integer_from_u16 u16 ;
        integer_from_u32 u32 ;
    }

    /// See [`Self::borrow_off_heap`].
    pub (super) unsafe fn borrow_off_heap_integer(&self) -> Val
    {
        // NOTE: Cast to i64 must happen before bitshift,
        //       as negative numbers require sign extension.
        let integer = self.inner.get() as i64 >> 4;
        Val::Integer(Int::Small(integer))
    }
}

#[cfg(test)]
mod tests
{
    use {super::*, proptest::proptest};

    macro_rules! roundtrip_small
    {
        { $($name:ident $method:ident $type:ty;)* } => {
            proptest!
            {
                $(
                    #[test]
                    fn $name(expected: $type)
                    {
                        let value = Value::$method(expected);
                        match value.borrow() {
                            Val::Integer(Int::Small(actual)) =>
                                assert_eq!(actual, expected as i64),
                            other =>
                                panic!("Unexpected val: {:?}", other),
                        }
                    }
                )*
            }
        };
    }

    roundtrip_small! {
        roundtrip_i8  integer_from_i8  i8  ;
        roundtrip_i16 integer_from_i16 i16 ;
        roundtrip_i32 integer_from_i32 i32 ;
        roundtrip_u8  integer_from_u8  u8  ;
        roundtrip_u16 integer_from_u16 u16 ;
        roundtrip_u32 integer_from_u32 u32 ;
    }
}
