//! Convert abstract syntax trees to bytecode.

use {
    crate::{bytecode::Unit, syntax::ast::Definition, value::Value},
    self::{collect_definitions::*, symbol::SymbolTable},
    std::{collections::HashMap, path::PathBuf, sync::Arc},
    thiserror::Error,
};

pub mod symbol;

mod collect_definitions;

/// Compilation result.
pub type Result<T> =
    std::result::Result<T, Error>;

/// Compilation error.
#[derive(Debug, Error)]
pub enum Error
{
    #[error("Too many constants")]
    TooManyConstants,

    #[error("Redefinition of `{0}`")]
    Redefinition(Arc<str>),
}

pub fn compile_unit(pathname: PathBuf, definitions: &[Definition])
    -> Result<Arc<Unit>>
{
    // Collect information about all definitions.
    let CollectedDefinitions{constants_allocated, init_phasers, globals} =
        collect_definitions(definitions)?;

    // Pre-allocate storage for constants that store globals.
    // These will be overwritten whilst compiling the definitions.
    let mut constants = vec![Value::undef(); constants_allocated as usize];

    Unit{pathname, constants, init_phasers, globals}
}
