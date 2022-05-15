//! Verification of bytecode procedures.
//!
//! To run at higher speed, the bytecode interpreter foregoes memory safety
//! based on the assumption that the bytecode does not do stupid things.
//! For example, if a jump instruction jumps to an out-of-bounds address,
//! or if an instruction references an out-of-bounds register,
//! this would not trigger an assertion in the interpreter.
//! The _bytecode verification_ algorithm checks that such behavior is absent.
//! Interpreting verified bytecode is guaranteed to be memory safe.

use {
    super::{Instruction, Procedure, Register},
    std::{fmt, ops::Deref},
    thiserror::Error,
};

/// Verified procedure.
///
/// See the [module documentation][`self`] for more information.
#[derive(Debug)]
pub struct Verified<'h>(Procedure<'h>);

/// Verification error.
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum Error
{
    #[error("There are no instructions")]
    NoInstructions,

    #[error("The last instruction is not a terminator")]
    LastInstructionIsNotATerminator,

    #[error("Invalid max_register value of {0:?}; needs at least {1:?}")]
    InvalidMaxRegister(Option<Register>, Option<Register>),
}

impl<'h> Verified<'h>
{
    /// Perform bytecode verification on a procedure.
    ///
    /// On success, the returned procedure can be interpreted safely.
    /// On failure, only the first encountered error is returned.
    pub fn new(procedure: Procedure<'h>) -> Result<Self, Error>
    {
        Self::verify_ends_with_terminator(&procedure)?;
        Self::verify_max_register(&procedure)?;
        Ok(Verified(procedure))
    }

    /// Forego bytecode verification.
    ///
    /// # Safety
    ///
    /// If [`Verified::new`] were called instead, it must have returned [`Ok`].
    /// Invalid usage can result in memory hazards during interpretation.
    pub unsafe fn new_unchecked(procedure: Procedure<'h>) -> Self
    {
        if cfg!(debug_assertions) {
            Self::new(procedure).unwrap()
        } else {
            Self(procedure)
        }
    }

    /// Verify that there are instructions
    /// and that the last one is a terminator.
    fn verify_ends_with_terminator(procedure: &Procedure) -> Result<(), Error>
    {
        let last_instruction =
            procedure.instructions.last()
            .ok_or(Error::NoInstructions)?;

        if !last_instruction.is_terminator() {
            return Err(Error::LastInstructionIsNotATerminator);
        }

        Ok(())
    }

    /// Verify that the `max_register` field is not too small.
    fn verify_max_register(procedure: &Procedure) -> Result<(), Error>
    {
        let actual_max_register =
            procedure.instructions.iter()
            .flat_map(Instruction::registers)
            .max();

        if procedure.max_register < actual_max_register {
            return Err(
                Error::InvalidMaxRegister(
                    procedure.max_register,
                    actual_max_register,
                )
            );
        }

        Ok(())
    }
}

impl<'h> Deref for Verified<'h>
{
    type Target = Procedure<'h>;

    fn deref(&self) -> &Self::Target
    {
        &self.0
    }
}

impl<'h> fmt::Display for Verified<'h>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        <Procedure as fmt::Display>::fmt(self, f)
    }
}

#[cfg(test)]
mod tests
{
    use {
        crate::heap::{BorrowRef, Heap, Mutator, PinnedRoot},
        super::*,
        std::assert_matches::assert_matches,
    };

    use Instruction as I;
    use Register as R;

    fn with_undef<F, R>(f: F) -> R
        where F: FnOnce(PinnedRoot) -> R
    {
        Heap::with(|heap| {
            let mutator = Mutator::new(heap);
            let undef = mutator.with_stack_roots(|[undef]| undef.pin());
            f(undef)
        })
    }

    #[test]
    fn example()
    {
        with_undef(|undef| {
            let procedure = Procedure{
                max_register: Some(R(2)),
                instructions: vec![
                    I::CopyRegister      {target: R(1), source: R(0)},
                    I::CopyConstant      {target: R(2), source: undef},
                    I::NumericAdd        {target: R(0), left: R(1), right: R(2)},
                    I::StringConcatenate {target: R(0), left: R(0), right: R(1)},
                    I::Return            {value: R(0)},
                ],
            };
            let result = Verified::new(procedure);
            assert_matches!(result, Ok(_));
        });
    }

    #[test]
    fn no_instructions()
    {
        let procedure = Procedure{
            max_register: None,
            instructions: Vec::new(),
        };
        let result = Verified::new(procedure);
        assert_matches!(result, Err(Error::NoInstructions));
    }

    #[test]
    fn last_instruction_is_not_a_terminator()
    {
        let procedure = Procedure{
            max_register: Some(R(1)),
            instructions: vec![
                I::Return       {value: R(0)},
                I::CopyRegister {target: R(0), source: R(1)},
            ],
        };
        let result = Verified::new(procedure);
        assert_matches!(result, Err(Error::LastInstructionIsNotATerminator));
    }

    #[test]
    fn invalid_max_register_none()
    {
        with_undef(|undef| {
            let procedure = Procedure{
                max_register: None,
                instructions: vec![
                    I::CopyConstant {target: R(0), source: undef},
                    I::Return       {value: R(0)},
                ],
            };
            let result = Verified::new(procedure);
            assert_matches!(result,
                Err(Error::InvalidMaxRegister(None, Some(R(0)))));
        });
    }

    #[test]
    fn invalid_max_register_some()
    {
        let procedure = Procedure{
            max_register: Some(R(0)),
            instructions: vec![
                I::CopyRegister {target: R(0), source: R(1)},
                I::Return       {value: R(0)},
            ],
        };
        let result = Verified::new(procedure);
        assert_matches!(result,
            Err(Error::InvalidMaxRegister(Some(R(0)), Some(R(1)))));
    }
}
