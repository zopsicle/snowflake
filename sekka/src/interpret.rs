use crate::bytecode::{Procedure, Verified};

pub fn interpret(procedure: &Verified)
{
    // SAFETY: Procedure was verified to be safe.
    unsafe { interpret_unverified(procedure) }
}

pub unsafe fn interpret_unverified(procedure: &Procedure)
{
}
