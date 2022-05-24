use {
    super::{Instruction, Procedure, Register},
    std::ops::Deref,
    thiserror::Error,
};

/// Procedure that can be interpreted safely.
///
/// To run at higher speed, the bytecode interpreter foregoes memory safety
/// based on the assumption that the bytecode does not do stupid things.
/// For example, if a jump instruction jumps to an out-of-bounds address,
/// or if an instruction references an out-of-bounds register,
/// this would not trigger an assertion in the interpreter.
/// The _bytecode verification_ algorithm checks that such behavior is absent.
#[derive(Debug)]
pub struct Verified(Procedure);

/// Error returned during verification.
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum VerifyError
{
    #[error("There are no instructions")]
    NoInstructions,

    #[error("The last instruction is not a terminator")]
    LastInstructionIsNotATerminator,

    #[error("Invalid max_register value of {0:?}; needs at least {1:?}")]
    InvalidMaxRegister(Option<Register>, Option<Register>),
}

impl Verified
{
    /// Verify a procedure.
    pub fn verify(procedure: Procedure) -> Result<Self, VerifyError>
    {
        Self::verify_ends_with_terminator(&procedure)?;
        Self::verify_max_register(&procedure)?;
        Ok(Self(procedure))
    }

    /// Forego procedure verification.
    ///
    /// # Safety
    ///
    /// If [`verify`][`Self::verify`] were used instead,
    /// it must have returned [`Ok`].
    pub unsafe fn new_unchecked(procedure: Procedure) -> Self
    {
        if cfg!(debug_assertions) {
            Self::verify(procedure).unwrap()
        } else {
            Self(procedure)
        }
    }

    /// Verify that there are instructions
    /// and that the last one is a terminator.
    fn verify_ends_with_terminator(procedure: &Procedure)
        -> Result<(), VerifyError>
    {
        let last_instruction =
            procedure.instructions.last()
            .ok_or(VerifyError::NoInstructions)?;

        if !last_instruction.is_terminator() {
            return Err(VerifyError::LastInstructionIsNotATerminator);
        }

        Ok(())
    }

    /// Verify that the `max_register` field is not too small.
    fn verify_max_register(procedure: &Procedure)
        -> Result<(), VerifyError>
    {
        let actual_max_register =
            procedure.instructions.iter()
            .flat_map(Instruction::registers)
            .max();

        if procedure.max_register < actual_max_register {
            return Err(
                VerifyError::InvalidMaxRegister(
                    procedure.max_register,
                    actual_max_register,
                )
            );
        }

        Ok(())
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

#[cfg(test)]
mod tests
{
    use {super::*, std::{assert_matches::assert_matches, sync::Weak}};

    use Instruction as I;
    use Register as R;
    use VerifyError as E;

    #[test]
    fn example()
    {
        let procedure = Procedure{
            unit: Weak::new(),
            max_register: Some(R(2)),
            instructions: vec![
                I::StringConcatenate {target: R(0), left: R(1), right: R(2)},
                I::Return            {result: R(0)},
            ],
            locations: Vec::new(),
        };
        let result = Verified::verify(procedure);
        assert_matches!(result, Ok(_));
    }

    #[test]
    fn no_instructions()
    {
        let procedure = Procedure{
            unit: Weak::new(),
            max_register: None,
            instructions: Vec::new(),
            locations: Vec::new(),
        };
        let result = Verified::verify(procedure);
        assert_matches!(result, Err(E::NoInstructions));
    }

    #[test]
    fn last_instruction_is_not_a_terminator()
    {
        let procedure = Procedure{
            unit: Weak::new(),
            max_register: Some(R(1)),
            instructions: vec![
                I::Return    {result: R(0)},
                I::LoadUndef {target: R(0)},
            ],
            locations: Vec::new(),
        };
        let result = Verified::verify(procedure);
        assert_matches!(result, Err(E::LastInstructionIsNotATerminator));
    }

    #[test]
    fn invalid_max_register_none()
    {
        let procedure = Procedure{
            unit: Weak::new(),
            max_register: None,
            instructions: vec![
                I::LoadUndef {target: R(0)},
                I::Return    {result: R(0)},
            ],
            locations: Vec::new(),
        };
        let result = Verified::verify(procedure);
        assert_matches!(result,
            Err(E::InvalidMaxRegister(None, Some(R(0)))));
    }

    #[test]
    fn invalid_max_register_some()
    {
        let procedure = Procedure{
            unit: Weak::new(),
            max_register: Some(R(0)),
            instructions: vec![
                I::LoadUndef {target: R(1)},
                I::Return    {result: R(0)},
            ],
            locations: Vec::new(),
        };
        let result = Verified::verify(procedure);
        assert_matches!(result,
            Err(E::InvalidMaxRegister(Some(R(0)), Some(R(1)))));
    }
}
