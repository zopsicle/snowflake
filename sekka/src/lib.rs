//! Sekka programming language.

#![doc(html_logo_url = "/sekka-manual/_static/logo.svg")]
#![feature(assert_matches)]
#![feature(get_mut_unchecked)]
#![feature(maybe_uninit_write_slice)]
#![feature(new_uninit)]
#![warn(missing_docs)]

use {
    self::unstable::{
        bytecode::Unit,
        compile::{self, compile_unit_from_source},
        value::Value,
    },
    std::{path::PathBuf, sync::Arc},
};

pub mod syntax;
pub mod unstable;

mod util;

/// Sekka virtual machine.
pub struct Sekka
{
    /// All units that have ever been loaded.
    ///
    /// Currently it is not possible to unload units,
    /// so this vector is only ever added to.
    units: Vec<Arc<Unit>>,
}

impl Sekka
{
    /// Create a new virtual machine.
    pub fn new() -> Self
    {
        Self{units: Vec::new()}
    }

    /// Compile and load a unit.
    pub fn compile(&mut self, pathname: PathBuf, source: &str)
        -> Result<PinnedUnit, compile::Error>
    {
        let unit = compile_unit_from_source(pathname, source)?;
        self.units.push(unit.clone());
        Ok(PinnedUnit{inner: unit})
    }
}

/// Reference to a Sekka unit.
pub struct PinnedUnit
{
    inner: Arc<Unit>,
}

/// Reference to a Sekka value.
pub struct PinnedValue
{
    inner: Value,
}
