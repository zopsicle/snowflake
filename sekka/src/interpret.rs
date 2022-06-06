// TODO: In object.rs, define Thunk and Lambda.
// TODO: Define function for forcing an object.
// TODO: Define function for interpreting a call.
#![allow(unused)]

//! Interpreter for the compiled representation.

use {
    crate::{
        anf::{Atom, Binding, Computation, Letrec},
        isolate::{Mutator, UnsafeHandle},
    },
    smallvec::SmallVec,
};

// In the below code a lot of arguments are passed all over the place.
// We use Greek letters for these arguments to keep them short and noticeable:
// μ for mutator, ε for environment, α for argument, β for binding.

/// Information about a call in tail position.
struct TailCall;

/* ========================================================================== */
/*                 BEGIN OF NO GARBAGE COLLECTION SAFE POINTS                 */
/* ========================================================================== */

// The code within this section must not enter a garbage collection safe point.
// That would potentially cause objects allocated for bindings to be collected,
// which would be disastrous as references to those objects
// may be returned to the caller or included in other objects.
// This section of code also doesn't really compute anything interesting,
// it just allocates memory, selects some objects, and returns what to do next.

/// Interpret a letrec.
unsafe fn interpret_letrec(
    μ: &Mutator,
    ε: *const UnsafeHandle,
    α: UnsafeHandle,
    letrec: &Letrec,
) -> Result<UnsafeHandle, TailCall>
{
    // Allocate memory for each binding.
    // No need to put these in scoped handles,
    // because we don't enter safe points in here.
    let βs: SmallVec<[UnsafeHandle; 2]> =
        letrec.bindings.iter()
        .map(|b| interpret_binding_alloc(μ, b))
        .collect();

    // Initialize the allocated memory for each binding.
    for (i, binding) in letrec.bindings.iter().enumerate() {
        let β = *βs.get_unchecked(i);
        interpret_binding_init(ε, α, βs.as_ptr(), β, binding);
    }

    interpret_computation(ε, α, βs.as_ptr(), &letrec.computation)
}

/// Obtain a handle to the object represented by an atom.
unsafe fn interpret_atom(
    ε: *const UnsafeHandle,
    α: UnsafeHandle,
    βs: *const UnsafeHandle,
    atom: &Atom,
) -> UnsafeHandle
{
    // IMPORTANT: Do not enter a garbage collection safe point here.

    match atom {
        Atom::Constant{handle}   => handle.get(),
        Atom::Environment{index} => *ε.add(*index),
        Atom::Argument           => α,
        Atom::Binding{index}     => *βs.add(*index),
    }
}

/// Allocate memory for a binding.
unsafe fn interpret_binding_alloc(μ: &Mutator, binding: &Binding)
    -> UnsafeHandle
{
    // IMPORTANT: Do not enter a garbage collection safe point here.

    match binding {
        Binding::Tuple{elements} => μ.alloc_tuple(elements.len()),
        _ => todo!(),
    }
}

/// Initialize the allocated memory for a binding.
unsafe fn interpret_binding_init(
    ε: *const UnsafeHandle,
    α: UnsafeHandle,
    βs: *const UnsafeHandle,
    β: UnsafeHandle,
    binding: &Binding,
)
{
    // IMPORTANT: Do not enter a garbage collection safe point here.

    match binding {

        Binding::Tuple{elements} =>
            Mutator::init_tuple_from_fn(β, elements.len(), |buf| {
                for (into, element) in buf.iter_mut().zip(elements) {
                    let handle = interpret_atom(ε, α, βs, element);
                    into.write(handle);
                }
            }),

        _ => todo!(),

    }
}

/// Interpret a computation.
unsafe fn interpret_computation(
    ε: *const UnsafeHandle,
    α: UnsafeHandle,
    βs: *const UnsafeHandle,
    computation: &Computation,
) -> Result<UnsafeHandle, TailCall>
{
    // IMPORTANT: Do not enter a garbage collection safe point here.

    match computation {

        Computation::Atom{atom} =>
            Ok(interpret_atom(ε, α, βs, atom)),

        Computation::Call{callee, argument} =>
            Err(todo!()),

    }
}

/* ========================================================================== */
/*                  END OF NO GARBAGE COLLECTION SAFE POINTS                  */
/* ========================================================================== */
