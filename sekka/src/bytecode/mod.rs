//! Bytecode instructions and procedures.

pub use self::verify::*;

use {crate::value::Value, std::path::PathBuf};

mod verify;

/// Compiled unit.
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

/// Sequence of instructions with metadata.
pub struct Procedure
{
    /// The instructions of the procedure.
    pub instructions: Vec<Instruction>,
}

/// Identifies an on-stack storage location.
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
