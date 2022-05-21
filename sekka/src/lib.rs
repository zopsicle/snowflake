//! Embeddable programming language.

#![warn(missing_docs)]

use {
    self::fiber::Fiber,
    std::{
        collections::HashMap,
        sync::{Mutex, atomic::{AtomicU64, Ordering::SeqCst}},
    },
};

mod bytecode;
mod fiber;
mod interpret;
mod value;

/// Identifies a fiber.
#[derive(Clone, Copy, Eq, Hash, PartialEq)]
pub struct FiberId(u64);

/// Sekka virtual machine state.
pub struct Sekka
{
    fibers: Mutex<HashMap<FiberId, Mutex<Fiber>>>,
    next_fiber_id: AtomicU64,
}

impl Sekka
{
    /// Create a new virtual machine.
    ///
    /// The virtual machine starts with no fibers.
    pub fn new() -> Self
    {
        Self{
            fibers: Mutex::new(HashMap::new()),
            next_fiber_id: AtomicU64::new(0),
        }
    }

    /// Spawn a fiber.
    ///
    /// The fiber starts with an empty call stack.
    pub fn spawn(&self) -> FiberId
    {
        let id = FiberId(self.next_fiber_id.fetch_add(1, SeqCst));
        let mut fibers = self.fibers.lock().unwrap();
        fibers.insert(id, Mutex::new(Fiber::new()));
        id
    }
}
