//! Bytecode instructions and procedures.

pub use self::{builder::*, verify::*};

use {
    crate::{syntax::location::Location, value::Value},
    std::{collections::HashMap, fmt, path::PathBuf, sync::{Arc, Weak}},
};

mod builder;
mod verify;

/* -------------------------------------------------------------------------- */
/*                          Bytecode data structures                          */
/* -------------------------------------------------------------------------- */

/// Information about a compiled unit.
#[derive(Default)]
pub struct Unit
{
    /// Pathname of the file from which the unit was compiled.
    ///
    /// This is used together with [`Location`]
    /// to provide location information in exceptions.
    ///
    /// [`Location`]: `crate::syntax::location::Location`
    pub pathname: PathBuf,

    /// Constants in the unit, in arbitrary order.
    ///
    /// These are referred to by [`Instruction::LoadConstant`].
    pub constants: Vec<Value>,

    /// Array of all init phasers.
    ///
    /// Each init phaser is compiled to a nilary subroutine.
    pub init_phasers: Vec<Constant>,

    /// Map of all globals.
    ///
    /// The keys of the map are the names of the globals.
    pub globals: HashMap<Arc<str>, Constant>,
}

/// Sequence of instructions with metadata.
pub struct Procedure
{
    /// The unit to which this procedure belongs.
    ///
    /// If the weak reference cannot be upgraded,
    /// calling the procedure fails with an exception
    /// telling the programmer that the unit was unloaded.
    pub unit: Weak<Unit>,

    /// Highest register accessed by any instruction.
    ///
    /// If no registers are used by any instructions,
    /// this field may be set to [`None`].
    pub max_register: Option<Register>,

    /// The instructions of the procedure.
    pub instructions: Vec<Instruction>,

    /// Associates each instruction with its location.
    ///
    /// There is only one element in this vector for each
    /// range of consecutive instructions with the same location.
    /// Use a binary search to find the location for a given instruction.
    pub locations: Vec<(usize, Location)>,
}

/// Indexes into [`Unit::constants`].
#[derive(Clone, Copy)]
pub struct Constant(pub u32);

/// Identifies an on-stack storage location.
#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub struct Register(pub u16);

/// Elementary instruction.
#[allow(missing_docs)]
#[derive(Debug)]
pub enum Instruction
{
    /// Load a constant to a register.
    LoadConstant{target: Register, constant: Constant},

    /// Load undef to a register.
    LoadUndef{target: Register},

    /// Convert the operands to strings and concatenate them.
    StringConcatenate{target: Register, left: Register, right: Register},

    /// Return to the caller with a result.
    Return{result: Register},

    /// Return to the caller with an exception.
    Throw{exception: Register},
}

/* -------------------------------------------------------------------------- */
/*                            Debug implementations                           */
/* -------------------------------------------------------------------------- */

impl fmt::Debug for Constant
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        // We explicitly *do not* want to use f.debug_tuple,
        // as that would insert noisy newlines with {:#?}.
        write!(f, "Constant({:?})", self.0)
    }
}

impl fmt::Debug for Register
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        // We explicitly *do not* want to use f.debug_tuple,
        // as that would insert noisy newlines with {:#?}.
        write!(f, "Register({:?})", self.0)
    }
}
