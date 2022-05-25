//! Symbols and symbol tables.

use {
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
    // If Ok, the parent is another symbol table.
    // If Err, the parent is the table of globals.
    parent: Result<&'a SymbolTable<'a>, &'a HashMap<Arc<str>, u32>>,

    symbols: HashMap<Arc<str>, Symbol>,
}

impl<'a> SymbolTable<'a>
{
    /// Create a symbol table with the table of globals as its parent.
    pub fn from_globals(globals: &'a HashMap<Arc<str>, u32>) -> Self
    {
        Self{parent: Err(globals), symbols: HashMap::new()}
    }

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
    Constant(u32),
}
