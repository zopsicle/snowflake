use {
    crate::syntax::location::Location,
    super::{Instruction, Procedure, Register, Unit},
    std::sync::Weak,
    thiserror::Error,
};

/// Convenient utility for generating procedures.
///
/// Keeps track of instructions, locations, and registers,
/// with convenient methods for common patterns.
pub struct Builder
{
    instructions: Vec<Instruction>,
    locations: Vec<(usize, Location)>,
    next_register: Register,
    max_register: Option<Register>,
}

/// Error returned during building.
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum BuildError
{
    #[error("Code requires too many registers")]
    TooManyRegisters,
}

impl Builder
{
    /// Create a new builder.
    pub fn new() -> Self
    {
        Self{
            instructions: Vec::new(),
            locations: Vec::new(),
            next_register: Register(0),
            max_register: None,
        }
    }

    /// Link the procedure.
    pub fn link(self, unit: Weak<Unit>) -> Procedure
    {
        Procedure{
            unit,
            max_register: self.max_register,
            instructions: self.instructions,
            locations: self.locations,
        }
    }

    /// Set the location to attach to subsequent instructions.
    pub fn set_location(&mut self, location: Location)
    {
        self.locations.push((self.instructions.len(), location));
    }

    /// Append an instruction to the procedure.
    pub fn build(&mut self, instruction: Instruction)
    {
        self.instructions.push(instruction);
    }

    /// Allocate a temporary and deallocate it when done.
    ///
    /// This allocates a new register that is not currently in use.
    /// The given function may use this register as it wishes.
    /// When the given function returns, the register is deallocated.
    pub fn with_register<F, R, E>(&mut self, f: F) -> Result<R, E>
        where F: FnOnce(&mut Self, Register) -> Result<R, E>
            , E: From<BuildError>
    {
        let register = self.next_register;

        self.max_register = self.max_register.max(Some(register));

        self.next_register.0 = self.next_register.0.checked_add(1)
            .ok_or(BuildError::TooManyRegisters)?;

        let result = f(self, register);

        self.next_register.0 -= 1;

        result
    }
}
