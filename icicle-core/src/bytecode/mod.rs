//! Working with bytecode instructions.

pub mod verify;

mod display;

/// Sequence of instructions.
pub struct Procedure
{
    /// The register with the highest number
    /// used by any of the instructions.
    pub max_register: Register,

    /// The instructions to execute.
    pub instructions: Vec<Instruction>,
}

/// Identifies a register.
pub struct Register(u16);

/// Instruction for the interpreter.
#[allow(missing_docs)]
pub enum Instruction
{
    /// Copy a value from one register into another.
    CopyRegister{
        target: Register,
        source: Register,
    },

    /// Copy a constant value into a register.
    CopyConstant{
        target: Register,
        source: Value,
    },

    /// Coerce left and right to numeric values,
    /// and write their sum into a register.
    NumericAdd{
        target: Register,
        left:   Register,
        right:  Register,
    },

    /// Coerce left and right to string values,
    /// and write their concatenation into a register.
    StringConcatenate{
        target: Register,
        left:   Register,
        right:  Register,
    },

    /// Return to the caller with a return value.
    Return{
        value: Register,
    },
}

// TODO: Move this out of the bytecode module.
#[derive(Debug)]
pub struct Value
{
    // TODO: Pointer to reference-counted value.
    _todo: usize,
}

#[cfg(test)]
mod tests
{
    use {super::*, std::mem::size_of};

    #[test]
    fn instruction_size()
    {
        assert!(
            size_of::<Instruction>() <= 16,
            "Try not to make instructions too big ({} B). \
             Consider moving large fields to the heap.",
            size_of::<Instruction>(),
        );
    }
}
