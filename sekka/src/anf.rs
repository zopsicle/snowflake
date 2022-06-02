/// Sequence of allocations followed by a tail call.
pub struct Block
{
    pub allocations: Box<[Allocation]>,
    pub continuation: Atom,
    pub argument: Atom,
}

pub enum Allocation
{
    /// Create a closure.
    Closure{
        /// Results of allocations to capture in the closure.
        environment: Box<[u32]>,

        /// The implementation of the closure.
        implementation: Box<Block>,
    },

    /// Create a non-empty list.
    Cons{
        /// The first element of the list.
        head: Atom,

        /// The remainder of the list, which must be a list.
        tail: Atom,
    },

    /// Create a tuple.
    Tuple{
        /// The elements of the tuple.
        elements: Box<[Atom]>,
    },
}

pub enum Atom
{
    /// A variable captured by the enclosing closure.
    Environment(u32),

    /// The argument passed to the enclosing closure.
    Argument,

    /// The result of an allocation in the enclosing block.
    Allocation(u32),

    /// The undef value.
    Undef,

    /// The empty list.
    Nil,
}
