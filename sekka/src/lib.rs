//! Embeddable programming language.
//!
//! Sekka source code is parsed and compiled into bytecode.
//! Bytecode is subsequently translated to Lua source code.
//! A Lua interpreter then executes the Lua source code.
//! This use of Lua is a temporary measure and an implementation detail.
//! It is in no way possible for Sekka embedders to access the Lua state.
//! A future version will not use Lua, and will interpret bytecode directly.

#![warn(missing_docs)]

use {std::ffi::CStr, thiserror::Error};

pub mod bytecode;

mod lower;
mod lua;

pub type Result<T> =
    std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error
{
    #[error("Lua: {0}")]
    Lua(#[from] lua::Error),
}

/// Sekka virtual machine.
pub struct Sekka
{
    lua: lua::State,
}

impl Sekka
{
    /// Create a new virtual machine.
    pub fn new() -> Result<Self>
    {
        let lua = lua::State::newstate()?;

        static RUNTIME: &[u8] = include_bytes!("runtime.lua");
        lua.do_string(CStr::from_bytes_with_nul(b"runtime.lua\0").unwrap(), RUNTIME)?;

        Ok(Self{lua})
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn f()
    {
        let _sekka = Sekka::new().unwrap();
    }
}
