//! Symbols and symbol tables.

use {
    crate::value::Value,
    std::{collections::{HashMap, hash_map::Entry::*}, sync::Arc},
    thiserror::Error,
};

/// Returned when trying to define a symbol that was already defined.
#[derive(Debug, Error)]
#[error("Redefinition of `{0}`")]
pub struct RedefinitionError(pub Arc<str>);

/// Associates names with symbols.
///
/// Symbol tables implement a hierarchy of scopes,
/// in the form of a borrowed linked list.
pub struct SymbolTable<'a>
{
    parent: Option<&'a SymbolTable<'a>>,
    symbols: HashMap<Arc<str>, Symbol>,
}

impl SymbolTable<'static>
{
    /// Create a symbol table with no parent.
    pub fn new_root() -> Self
    {
        Self{parent: None, symbols: HashMap::new()}
    }
}

impl<'a> SymbolTable<'a>
{
    /// Define a new symbol in this scope.
    pub fn define(&mut self, name: Arc<str>, symbol: Symbol)
        -> Result<(), RedefinitionError>
    {
        match self.symbols.entry(name) {
            Occupied(entry) => Err(RedefinitionError(entry.key().clone())),
            Vacant(entry) => Ok({ entry.insert(symbol); }),
        }
    }
}

/// Describes a definition referred to by some name.
pub enum Symbol
{
    /// The name was defined as a constant.
    ///
    /// This is also used for subroutine definitions.
    Constant(Value),
}
