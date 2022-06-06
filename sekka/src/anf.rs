use {crate::isolate::UnsafeHandle, std::sync::Arc};

/// Sequence of mutually recursive bindings followed by a computation.
pub struct Letrec
{
    pub bindings: Vec<Binding>,
    pub computation: Computation,
}

/// Trivial expression that requires no allocation.
pub enum Atom
{
    Constant{
        handle: UnsafeHandle,
    },

    Environment{
        index: usize,
    },

    Argument,

    Binding{
        /// Indexes into [`Letrec::bindings`].
        index: usize,
    },
}

/// Trivial expression that requires allocation.
pub enum Binding
{
    Tuple{
        elements: Vec<Atom>,
    },

    Thunk{
        environment: Vec<Atom>,
        body: Arc<Letrec>,
    },

    Lambda{
        environment: Vec<Atom>,
        body: Arc<Letrec>,
    },
}

/// Expression that reduces by non-trivial rules.
pub enum Computation
{
    Call{
        callee: Atom,
        argument: Atom,
    },
}
