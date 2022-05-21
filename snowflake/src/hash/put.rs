use {crate::basename::Basename, super::Blake3, std::ffi::CStr};

/// Convenient methods for writing values.
///
/// In contrast with the [`Hash`][`std::hash::Hash`] trait,
/// these methods are stable across platforms and versions.
/// To aid in avoiding instability, the methods are all named differently.
/// Changing a type would hence result in a type error, unlike with a trait.
#[allow(missing_docs)]
impl Blake3
{
    // NOTE: See the manual chapter on avoiding hash collisions.

    pub fn put_bool(&mut self, value: bool) -> &mut Self
    {
        self.put_u8(value as u8)
    }

    pub fn put_u8(&mut self, value: u8) -> &mut Self
    {
        self.update(&[value])
    }

    pub fn put_u64(&mut self, value: u64) -> &mut Self
    {
        self.update(&value.to_le_bytes())
    }

    pub fn put_usize(&mut self, value: usize) -> &mut Self
    {
        self.put_u64(value as u64)
    }

    pub fn put_bytes(&mut self, value: &[u8]) -> &mut Self
    {
        self.put_usize(value.len()).update(value)
    }

    pub fn put_cstr(&mut self, value: &CStr) -> &mut Self
    {
        self.update(value.to_bytes_with_nul())
    }

    pub fn put_basename(&mut self, value: &Basename) -> &mut Self
    {
        self.update(value.as_bytes())
    }
}
