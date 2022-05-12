//! Working with bytecode instructions.

pub mod verify;

mod display;

/// Sequence of instructions.
#[derive(Debug)]
pub struct Procedure
{
    /// The register with the highest number
    /// used by any of the instructions.
    ///
    /// If no registers are used, this is [`None`].
    pub max_register: Option<Register>,

    /// The instructions to execute.
    pub instructions: Vec<Instruction>,
}

/// Identifies a register.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Register(u16);

/// Instruction for the interpreter.
#[allow(missing_docs)]
#[derive(Debug)]
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
        source: /* TODO: Value */ usize,
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

impl Instruction
{
    /// Whether the instruction is a terminator.
    ///
    /// A terminator unconditionally transfers control;
    /// it never continues to the subsequent instruction
    /// (except if it's a jump equivalent to a no-op).
    pub fn is_terminator(&self) -> bool
    {
        match self {
            // Terminators.
            Self::Return{..} => true,

            // Non-terminators.
            Self::CopyRegister{..}      => false,
            Self::CopyConstant{..}      => false,
            Self::NumericAdd{..}        => false,
            Self::StringConcatenate{..} => false,
        }
    }

    /// The registers used by the instruction.
    ///
    /// The returned iterator yields the registers in arbitrary order.
    /// It yields the same register multiple times
    /// if it appears multiple times in the instruction.
    pub fn registers(&self) -> impl Iterator<Item=Register>
    {
        macro_rules! chain
        {
            ($sub:expr $(, $subs:expr)* $(,)?) => {
                IntoIterator::into_iter($sub)$(.chain($subs))*
            };
        }
        match self {
            Self::CopyRegister{target, source} =>
                chain!(Some(*target), Some(*source), None),
            Self::CopyConstant{target, source: _} =>
                chain!(Some(*target), None, None),
            Self::NumericAdd{target, left, right} =>
                chain!(Some(*target), Some(*left), Some(*right)),
            Self::StringConcatenate{target, left, right} =>
                chain!(Some(*target), Some(*left), Some(*right)),
            Self::Return{value} =>
                chain!(Some(*value), None, None),
        }
    }
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
