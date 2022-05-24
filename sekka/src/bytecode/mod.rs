//! Bytecode instructions and procedures.

pub use self::verify::*;

use {
    crate::value::Value,
    std::{collections::HashMap, path::PathBuf, sync::{Arc, Weak}},
};

mod verify;

/// Information about a compiled unit.
pub struct Unit
{
    /// Pathname of the file from which the unit was compiled.
    pub pathname: PathBuf,

    /// Constants in the unit, in arbitrary order.
    ///
    /// These are referred to by [`Instruction::CopyConstant`].
    pub constants: Vec<Value>,

    /// Array of all init phasers.
    ///
    /// Each init phaser is compiled to a nilary subroutine.
    /// These index into [`constants`][`Self::constants`].
    pub init_phasers: Vec<u32>,

    /// Map of all globals.
    ///
    /// The keys of the map are the names of the globals.
    /// The values of the map index into [`constants`][`Self::constants`].
    pub globals: HashMap<Arc<str>, u32>,
}

/// Sequence of instructions with metadata.
pub struct Procedure
{
    /// The unit to which this procedure belongs.
    pub unit: Weak<Unit>,

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
    CopyConstant{target: Register, constant: u32},

    /// Convert the operands to strings and concatenate them.
    StringConcatenate{target: Register, left: Register, right: Register},

    /// Return to the caller with a result.
    Return{result: Register},

    /// Return to the caller with an exception.
    Throw{exception: Register},
}
