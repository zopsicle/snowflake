//! Verification of bytecode procedures.
//!
//! To run at higher speed, the bytecode interpreter foregoes memory safety
//! based on the assumption that the bytecode does not do stupid things.
//! For example, if a jump instruction jumps to an out-of-bounds address,
//! or if an instruction references an out-of-bounds register,
//! this would not trigger an assertion in the interpreter.
//! The _bytecode verification_ algorithm checks that such behavior is absent.
//! Interpreting verified bytecode is guaranteed to be memory safe.

use {super::Procedure, std::{fmt, ops::Deref}};

/// Verified procedure.
pub struct Verified(Procedure);

impl Deref for Verified
{
    type Target = Procedure;

    fn deref(&self) -> &Self::Target
    {
        &self.0
    }
}

impl fmt::Display for Verified
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        <Procedure as fmt::Display>::fmt(self, f)
    }
}
