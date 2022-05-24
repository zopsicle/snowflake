use {super::Procedure, std::ops::Deref};

/// Procedure that can be interpreted safely.
pub struct Verified(Procedure);

impl Deref for Verified
{
    type Target = Procedure;

    fn deref(&self) -> &Self::Target
    {
        &self.0
    }
}
