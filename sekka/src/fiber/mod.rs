use self::call_stack::*;

mod call_stack;

/// Fiber state.
pub struct Fiber
{
    call_stack: CallStack,
}

impl Fiber
{
    /// Create a fiber with an empty call stack.
    pub fn new() -> Self
    {
        Self{call_stack: CallStack::new()}
    }
}
