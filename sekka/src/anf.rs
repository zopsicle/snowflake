//! Compiled representation of expressions.

use {crate::isolate::UnsafeHandle, std::{cell::Cell, sync::Arc}};

/// Sequence of mutually recursive bindings followed by a computation.
pub struct Letrec
{
    /// The bindings that the computation may close over.
    pub bindings: Vec<Binding>,

    /// The computation that yields the result of this letrec.
    pub computation: Computation,
}

/// Trivial expression that requires no allocation.
#[allow(missing_docs)]
pub enum Atom
{
    /// Yield a hardcoded object.
    ///
    /// This is a cell because the garbage collector
    /// must update the handle after moving the object.
    Constant{handle: Cell<UnsafeHandle>},

    /// Yield a variable captured by the current closure.
    ///
    /// Each closure contains an array of captured variables,
    /// which is known as the environment of the closure.
    /// This index indexes into that array.
    Environment{index: usize},

    /// Yield the argument passed to the current lambda.
    ///
    /// This is meaningless within thunks,
    /// and would yield an arbitrary value.
    Argument,

    /// Yield the result of a binding in the letrec that contains this atom.
    ///
    /// Indexes into [`Letrec::bindings`].
    Binding{index: usize},
}

/// Trivial expression that requires allocation.
#[allow(missing_docs)]
pub enum Binding
{
    /// Allocate and initialize a tuple with the given elements.
    Tuple{elements: Vec<Atom>},

    /// Allocate and initialize a tuple with the given environment and body.
    Thunk{environment: Vec<Atom>, body: Arc<Letrec>},

    /// Allocate and initialize a lambda with the given environment and body.
    Lambda{environment: Vec<Atom>, body: Arc<Letrec>},
}

/// Expression that possibly reduces by non-trivial rules.
#[allow(missing_docs)]
pub enum Computation
{
    /// Yield an atom.
    Atom{atom: Atom},

    /// Evaluate the callee and perform a tail call.
    Call{callee: Atom, argument: Atom},
}
