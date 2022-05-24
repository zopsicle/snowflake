use {
    crate::syntax::ast::Definition,
    super::{Error, Result},
    std::{collections::{HashMap, hash_map::Entry::*}, sync::Arc},
};

/// Information collected about definitions.
pub struct CollectedDefinitions
{
    /// The number of constants allocated.
    pub constants_allocated: u32,

    /// Corresponds to [`Unit::init_phasers`].
    pub init_phasers: Vec<u32>,

    /// Corresponds to [`Unit::globals`].
    pub globals: HashMap<Arc<str>, u32>,
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
    let mut init_phasers = Vec::new();
    let mut globals = HashMap::new();

    let mut allocate_constant = || -> Result<u32> {
        constants_allocated =
            constants_allocated.checked_add(1)
            .ok_or(Error::TooManyConstants)?;
        Ok(constants_allocated - 1)
    };

    for definition in definitions {
        match definition {

            Definition::InitPhaser{..} => {
                let constant = allocate_constant()?;
                init_phasers.push(constant);
            },

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

    Ok(CollectedDefinitions{constants_allocated, init_phasers, globals})
}
