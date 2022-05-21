//! Embeddable programming language.
//!
//! Sekka source code is parsed and compiled into bytecode.
//! Bytecode is subsequently translated to Lua source code.
//! A Lua interpreter then executes the Lua source code.
//! This use of Lua is a temporary measure and an implementation detail.
//! It is in no way possible for Sekka embedders to access the Lua state.
//! A future version will not use Lua, and will interpret bytecode directly.

#![feature(let_else)]
#![warn(missing_docs)]

use {
    lua_sys::{lua_State, luaL_newstate, lua_close},
    std::{alloc::{Layout, handle_alloc_error}, ptr::NonNull},
};

pub mod bytecode;

mod lower;

/// Sekka virtual machine.
pub struct Sekka
{
    lua: NonNull<lua_State>,
}

impl Sekka
{
    /// Create a new virtual machine.
    pub fn new() -> Self
    {
        // SAFETY: This function is always safe to call.
        let lua = unsafe { luaL_newstate() };

        let Some(lua) = NonNull::new(lua)
            // We have to pass a layout to handle_alloc_error.
            else { handle_alloc_error(Layout::new::<()>()) };

        Self{lua}
    }
}

impl Drop for Sekka
{
    fn drop(&mut self)
    {
        unsafe { lua_close(self.lua.as_ptr()); }
    }
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn f()
    {
        let _sekka = Sekka::new();
    }
}
