use {
    crate::{syntax::ast::Definition, unstable::bytecode::Constant},
    super::{Error, Result},
    std::{collections::{HashMap, hash_map::Entry::*}, sync::Arc},
};

/// Information collected about definitions.
pub struct CollectedDefinitions
{
    /// The number of constants allocated.
    pub constants_allocated: u32,

    /// Corresponds to [`Unit::globals`].
    pub globals: HashMap<Arc<str>, Constant>,
}

/// Collected information about definitions.
///
/// This loops through all definitions in a unit
/// and collects information about what they define.
/// It also raises errors about definitions having the same name.
pub fn collect_definitions(definitions: &[Definition])
    -> Result<CollectedDefinitions>
{
    let mut constants_allocated = 0u32;
    let mut globals = HashMap::new();

    let mut allocate_constant = || -> Result<Constant> {
        constants_allocated =
            constants_allocated.checked_add(1)
            .ok_or(Error::TooManyConstants)?;
        Ok(Constant(constants_allocated - 1))
    };

    for definition in definitions {
        match definition {

            Definition::InitPhaser{..} =>
                (/* nothing to collect */),

            Definition::Subroutine{name, ..} =>
                match globals.entry(name.clone()) {
                    Occupied(entry) => {
                        let name = entry.key().clone();
                        return Err(Error::Redefinition(name));
                    },
                    Vacant(entry) => {
                        let constant = allocate_constant()?;
                        entry.insert(constant);
                    },
                },

        }
    }

    Ok(CollectedDefinitions{constants_allocated, globals})
}
