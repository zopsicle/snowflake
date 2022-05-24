pub use self::verify::*;

use {crate::value::Value, std::{path::PathBuf, sync::Weak}};

mod verify;

pub struct Unit
{
    /// Filepath from which the unit was compiled.
    pub filepath: PathBuf,

    /// Procedures in the unit, in arbitrary order.
    ///
    /// This includes not only top-level subroutines,
    /// but also implementations of lambda subroutines.
    /// Subroutine values refer to procedures by their index.
    pub procedures: Vec<Verified>,
}

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
