//! Intermediate representation of programs.

pub use self::lower::*;

use {crate::syntax::location::Location, std::sync::Arc};

mod lower;

/// Convenient interface for generating IR.
///
/// Because of their ubiquitous use, builders are usually abbreviated `b`.
pub struct Builder
{
    location: Location,
}

macro_rules! build_doc
{
    ($a:literal, $name:literal) => {
        concat!(
            "Build ", $a, " [`", $name, "`][`Instruction::",
            $name, "`] instruction.",
        )
    };
}

impl Builder
{
    /// Set the location attached to subsequent instructions.
    pub fn set_location(&mut self, location: Location)
    {
        self.location = location;
    }

    fn build_instruction(&mut self, instruction: Instruction) -> Value
    {
        todo!()
    }

    #[doc = build_doc!("a", "NumericAdd")]
    pub fn build_numeric_add(&mut self, left: Value, right: Value) -> Value
    {
        self.build_instruction(Instruction::NumericAdd{left, right})
    }

    #[doc = build_doc!("a", "StringConcatenate")]
    pub fn build_string_concatenate(&mut self, left: Value, right: Value)
        -> Value
    {
        self.build_instruction(Instruction::StringConcatenate{left, right})
    }

    #[doc = build_doc!("a", "Return")]
    pub fn build_return(&mut self, result: Value)
    {
        self.build_instruction(Instruction::Return{result});
    }
}

/// Variables or constants.
#[allow(missing_docs)]
pub enum Value
{
    String(Arc<[u8]>),
}

/// Elementary instructions.
#[allow(missing_docs)]
pub enum Instruction
{
    /// Convert the operands to numbers and add them.
    NumericAdd{left: Value, right: Value},

    /// Convert the operands to strings and concatenate them.
    StringConcatenate{left: Value, right: Value},

    /// Return to the caller with the operand as the return value.
    Return{result: Value},
}
