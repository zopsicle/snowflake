pub use self::verify::*;

use crate::value::Value;

mod verify;

pub struct Procedure
{
    pub instructions: Vec<Instruction>,
}

#[derive(Clone, Copy)]
pub struct Register(pub u16);

/// Elementary instruction.
#[allow(missing_docs)]
pub enum Instruction
{
    /// Copy a constant to a register.
    CopyConstant{target: Register, constant: Value},

    /// Convert the operands to strings and concatenate them.
    StringConcatenate{target: Register, left: Register, right: Register},

    /// Return to the caller with a result.
    Return{result: Register},

    /// Return to the caller with an exception.
    Throw{exception: Register},
}
