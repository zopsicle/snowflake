// NOTE: The current implementation is very inefficient.
// A future version should not use nested arrays,
// but just one single array with value/metadata unions.

use {crate::{bytecode::{Register, Verified}, value::Value}, std::sync::Arc};

/// Call stack of a fiber.
pub struct CallStack
{
    stack_frames: Vec<StackFrame>,
}

struct StackFrame
{
    registers: Vec<Value>,
    procedure: Arc<Verified>,
    program_counter: usize,
}

impl CallStack
{
    /// Create an empty call stack.
    pub fn new() -> Self
    {
        Self{stack_frames: Vec::new()}
    }

    /// Push a stack frame for the given procedure.
    pub fn push(&mut self, procedure: Arc<Verified>)
    {
        let registers =
            procedure.max_register
            .map(|Register(i)| 1 + i as usize)
            .unwrap_or(0);
        let stack_frame = StackFrame{
            registers: vec![Value::undef(); registers],
            procedure,
            program_counter: 0,
        };
        self.stack_frames.push(stack_frame);
    }
}
