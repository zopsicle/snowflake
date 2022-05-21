use {super::Procedure, std::ops::Deref, thiserror::Error};

/// Bytecode that has been verified.
///
/// Bytecode verification ensures that all jump targets and register accesses
/// of instructions are in bounds, so the bytecode can be interpreted safely.
pub struct Verified(Procedure);

/// Error from bytecode verification.
#[derive(Debug, Error)]
pub enum VerifyError
{
}

impl Verified
{
    /// Verify bytecode.
    pub fn verify(procedure: Procedure) -> Result<Self, VerifyError>
    {
        todo!()
    }

    /// Forego bytecode verification.
    pub unsafe fn new_unchecked(procedure: Procedure) -> Self
    {
        Self(procedure)
    }
}

impl Deref for Verified
{
    type Target = Procedure;

    fn deref(&self) -> &Self::Target
    {
        &self.0
    }
}
