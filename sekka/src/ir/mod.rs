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
    instructions: Vec<(Location, Register, Instruction)>,
    next_register: Register,
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
    /// Create a new builder with an empty block.
    pub fn new() -> Self
    {
        Self{
            location: Location{offset: 0},
            instructions: Vec::new(),
            next_register: Register(0),
        }
    }

    /// Finish building, returning the built procedure.
    pub fn finish(self) -> Vec<(Location, Register, Instruction)>
    {
        self.instructions
    }

    /// Set the location attached to subsequent instructions.
    pub fn set_location(&mut self, location: Location)
    {
        self.location = location;
    }

    fn build_instruction(&mut self, instruction: Instruction) -> Value
    {
        let result = self.next_register;
        self.instructions.push((self.location, result, instruction));
        self.next_register.0 += 1;
        Value::Register(result)
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

/// Identifies the result of an instruction.
#[derive(Clone, Copy)]
#[derive(Debug)]
pub struct Register(pub u64);

/// Variables or constants.
#[allow(missing_docs)]
#[derive(Debug)]
pub enum Value
{
    Register(Register),
    String(Arc<[u8]>),
}

/// Elementary instructions.
#[allow(missing_docs)]
#[derive(Debug)]
pub enum Instruction
{
    /// Convert the operands to numbers and add them.
    NumericAdd{left: Value, right: Value},

    /// Convert the operands to strings and concatenate them.
    StringConcatenate{left: Value, right: Value},

    /// Return to the caller with the operand as the return value.
    Return{result: Value},
}
