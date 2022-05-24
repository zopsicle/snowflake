use {super::Procedure, std::ops::Deref, thiserror::Error};

/// Procedure that can be interpreted safely.
pub struct Verified(Procedure);

/// Error returned during verification.
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum VerifyError
{
}

impl Verified
{
    pub fn verify() -> Result<Self, VerifyError>
    {
        todo!()
    }

    /// Forego procedure verification.
    ///
    /// # Safety
    ///
    /// If [`verify`][`Self::verify`] were used instead,
    /// it must have returned [`Ok`].
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
