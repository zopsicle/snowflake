use {
    crate::istring::IStr,
    super::{OnHeapHeader, Val, Value, off_heap_tag, on_heap_tag},
    std::{
        alloc::LayoutError,
        mem::{MaybeUninit, align_of, size_of},
        num::NonZeroU64,
        slice,
    },
    thiserror::Error,
};

/// Representation of on-heap strings.
#[repr(C)]
struct OnHeapString
{
    len: usize,
    bytes: [u8; 0 /* self.len + 1 */],
}

/// Returned when a string is too long to be created.
#[derive(Debug, Error)]
#[error("String is too long to be created")]
pub struct StringLenError
{
    _priv: (),
}

/// Working with string values.
impl Value
{
    /// Create a string from the bytes that make it up.
    ///
    /// The bytes must not include the terminating nul;
    /// it will be added automatically by this method.
    pub fn string_from_bytes(bytes: &[u8]) -> Result<Self, StringLenError>
    {
        // SAFETY: The function initializes the buffer.
        unsafe {
            Self::string_from_fn(bytes.len(), |buf| {
                MaybeUninit::write_slice(buf, bytes);
            })
        }
    }

    /// Create a string using a function that initializes it.
    ///
    /// Memory is allocated for the string,
    /// which the given function must initialize.
    /// The function must not write the terminating nul;
    /// it will be added automatically by this method.
    ///
    /// # Safety
    ///
    /// When the given function returns,
    /// the entire buffer must be initialized.
    pub unsafe fn string_from_fn<F>(len: usize, f: F)
        -> Result<Self, StringLenError>
        where F: FnOnce(&mut [MaybeUninit<u8>])
    {
        if len <= 6 {
            Ok(Self::string_from_fn_off_heap(len, f))
        } else {
            Self::string_from_fn_on_heap(len, f)
        }
    }

    unsafe fn string_from_fn_off_heap<F>(len: usize, f: F) -> Self
        where F: FnOnce(&mut [MaybeUninit<u8>])
    {
        // Zero-initialize so that the padding bits are zero.
        // Dead store elimination should optimize this.
        let mut buf = [MaybeUninit::zeroed(); 8];
        f(&mut buf[0 .. len]);

        // Convert the buffer using native-endian to avoid byte swap,
        // as we pointer-cast this in `borrow_off_heap_string`.
        let buf = MaybeUninit::array_assume_init(buf);
        let buf = u64::from_ne_bytes(buf);

        // On little-endian systems, the least significant byte
        // precedes the payload, so we have to adjust the buffer.
        #[cfg(target_endian = "little")]
        let buf = buf << 8;

        // Set the length nibble and tag bits.
        let payload = buf | (len as u64) << 4;
        Self::from_off_heap(payload | off_heap_tag::STRING)
    }

    unsafe fn string_from_fn_on_heap<F>(len: usize, f: F)
        -> Result<Self, StringLenError>
        where F: FnOnce(&mut [MaybeUninit<u8>])
    {
        // If the string is too large, return an error.
        const ERR: StringLenError = StringLenError{_priv: ()};

        // Include sufficient space for the terminating nul.
        let payload_size =
            size_of::<OnHeapString>()
            .checked_add(len).ok_or(ERR)?
            .checked_add(1).ok_or(ERR)?;
        let payload_align = align_of::<OnHeapString>();

        // Initialize the payload of the string value.
        let init_payload = |payload: *mut ()| {
            let payload = payload.cast::<OnHeapString>();

            // Initialize the len field.
            (*payload).len = len;

            // Initialize the bytes and terminating nul.
            let bytes = (*payload).bytes.as_mut_ptr() as *mut MaybeUninit<u8>;
            let bytes = slice::from_raw_parts_mut(bytes, len + 1);
            f(&mut bytes[0 .. len]);
            bytes[len].write(0);

            // Return the extra word.
            on_heap_tag::STRING
        };

        // Allocate memory for the string value and initialize it.
        Self::new_on_heap(payload_size, payload_align, init_payload)
            .map_err(|_: LayoutError| ERR)
    }

    /// See [`Self::borrow_off_heap`].
    pub (super) unsafe fn borrow_off_heap_string(&self) -> Val
    {
        let ptr = &self.inner as *const NonZeroU64 as *const u8;

        // On little-endian systems, the least significant byte
        // precedes the payload, so we have to adjust the pointer.
        #[cfg(target_endian = "little")]
        let ptr = ptr.add(1);

        let len = (self.inner.get() >> 4 & 0b111) as usize;
        let bytes = slice::from_raw_parts(ptr, len + 1);
        let istr = IStr::from_bytes_with_nul_unchecked(bytes);
        Val::String(istr)
    }

    /// See [`Self::borrow_on_heap`].
    pub (super) unsafe fn borrow_on_heap_string(on_heap: &OnHeapHeader) -> Val
    {
        let on_heap = on_heap as *const OnHeapHeader;
        let payload = on_heap.add(1) as *const OnHeapString;
        let bytes = slice::from_raw_parts(
            (*payload).bytes.as_ptr(),
            (*payload).len + 1,
        );
        let istr = IStr::from_bytes_with_nul_unchecked(bytes);
        Val::String(istr)
    }

    /// See [`Self::drop_on_heap`].
    pub (super) unsafe fn drop_on_heap_string(_on_heap: &OnHeapHeader)
    {
        // Strings only contain length and bytes.
        // There is nothing that needs dropping here.
    }
}

#[cfg(test)]
mod tests
{
    use {
        super::*,
        proptest::{
            collection::vec as pvec,
            num::u8::ANY as pu8,
            proptest,
        },
    };

    proptest!
    {
        #[test]
        fn roundtrip_small(expected in pvec(pu8, 0 ..= 6))
        {
            let value = Value::string_from_bytes(&expected).unwrap();
            match value.borrow() {
                Val::String(istr) => assert_eq!(istr.as_bytes(), &expected),
                other => panic!("Unexpected val: {:?}", other),
            }
        }

        #[test]
        fn roundtrip_large(expected in pvec(pu8, 7 .. 100))
        {
            let value = Value::string_from_bytes(&expected).unwrap();
            match value.borrow() {
                Val::String(istr) => assert_eq!(istr.as_bytes(), &expected),
                other => panic!("Unexpected val: {:?}", other),
            }
        }
    }
}

