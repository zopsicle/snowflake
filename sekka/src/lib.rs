#![doc(html_logo_url = "/sekka-manual/_static/logo.svg")]
#![feature(extern_types)]
#![feature(once_cell)]
#![warn(missing_docs)]

use self::backend::Backend;

pub mod ir;
pub mod syntax;

mod backend;

/// Sekka virtual machine with its own garbage-collected heap.
pub struct Isolate
{
    backend: Backend,
}

impl Isolate
{
    /// Create a new isolate.
    pub fn new() -> Self
    {
        Self{backend: Backend::new()}
    }

    pub fn run(
        &self,
        instructions: &[
            (
                syntax::location::Location,
                ir::Register,
                ir::Instruction,
            )
        ],
    )
    {
        let mut w = String::new();
        for (_location, result, instruction) in instructions {
            backend::lower::lower_instruction(&mut w, *result, instruction);
        }
        println!("{}", w);
    }
}
