use {
    super::Hash,
    blake3_c_rust_bindings::Hasher,
    std::io::{self, IoSlice, Write},
};

/// BLAKE3 cryptographic hash function.
///
/// The [`Write`] impl calls [`update`] for each incoming buffer.
/// The methods on the [`Write`] impl never return an error.
///
/// [`update`]: `Self::update`
pub struct Blake3(Hasher);

impl Blake3
{
    /// Create a new hasher.
    pub fn new() -> Self
    {
        Self(Hasher::new())
    }

    /// Add data to the hasher.
    ///
    /// Returns `self` for convenience.
    pub fn update(&mut self, buf: &[u8]) -> &mut Self
    {
        self.0.update(buf);
        self
    }

    /// Extract the hash from the hasher.
    pub fn finalize(&self) -> Hash
    {
        let mut hash = Hash([0; 32]);
        self.0.finalize(&mut hash.0);
        hash
    }
}

impl Write for Blake3
{
    fn write(&mut self, buf: &[u8]) -> io::Result<usize>
    {
        self.update(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()>
    {
        Ok(())
    }

    fn write_vectored(&mut self, bufs: &[IoSlice]) -> io::Result<usize>
    {
        let mut len = 0usize;
        for buf in bufs {
            if let Some(new_len) = len.checked_add(buf.len()) {
                len = new_len;
                self.update(buf);
            } else {
                break;
            }
        }
        Ok(len)
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()>
    {
        self.update(buf);
        Ok(())
    }
}
