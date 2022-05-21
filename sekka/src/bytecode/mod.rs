//! Bytecode data structures.

pub use self::verify::*;

mod verify;

/// Sequence of instructions.
pub struct Procedure
{
    /// The highest register used by any instruction.
    ///
    /// If no registers are used, this is [`None`].
    pub max_register: Option<Register>,

    /// Instructions to interpret.
    pub instructions: Vec<Instruction>,
}

/// Identifies a register.
#[derive(Clone, Copy)]
pub struct Register(pub u16);

/// Identifies an instruction.
#[derive(Clone, Copy)]
pub struct Label(pub u16);

/// Elementary instruction.
#[allow(missing_docs)]
pub enum Instruction
{
    /// Copy value from source to target.
    Copy{target: Register, source: Register},

    /// Jump to target.
    Jump{target: Label},

    /// Convert condition to Boolean and jump to target if true.
    JumpIf{target: Label, condition: Register},

    /// Return to the caller with value.
    Return{value: Register},

    /// Convert operand to Boolean and store in target.
    ToBoolean{target: Register, operand: Register},

    /// Convert operand to numeric and store in target.
    ToNumeric{target: Register, operand: Register},

    /// Convert operand to string and store in target.
    ToString{target: Register, operand: Register},
}
