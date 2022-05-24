//! Interpreting bytecode.

use {
    crate::{
        bytecode::{Instruction, Register, Unit},
        value::{StringFromBytesError, ToStringError, Value},
    },
    std::{mem::{MaybeUninit, replace}, sync::Arc},
    thiserror::Error,
};

/* -------------------------------------------------------------------------- */
/*                                Interpreting                                */
/* -------------------------------------------------------------------------- */

/// How to change the call stack after interpreting instructions.
pub enum CallStackDelta
{
    /// Pop the stack frame with a return value.
    Return(Value),

    /// Pop the stack frame with an exception.
    Throw(Value),
}

/// Interpret instructions until the call stack needs to be changed.
///
/// # Safety
///
/// There must be sufficient registers.
/// The instructions must not jump out of bounds.
/// The instructions must not refer to out of bounds constants.
pub unsafe fn interpret(
    unit: &Unit,
    registers: *mut Value,
    mut program_counter: *const Instruction,
) -> CallStackDelta
{
    let move_register = |register: Register| {
        let register = &mut *registers.add(register.0 as usize);
        replace(register, Value::undef())
    };

    let borrow_register = |register: Register| {
        &*registers.add(register.0 as usize)
    };

    let clone_register = |register: Register| {
        borrow_register(register).clone()
    };

    let set_register = |register: Register, value: Value| {
        *registers.add(register.0 as usize) = value;
    };

    loop {

        match &*program_counter {

            Instruction::CopyConstant{target, constant} => {
                let constant = unit.constants.get_unchecked(*constant as usize);
                set_register(*target, constant.clone());
                program_counter = program_counter.add(1);
            },

            Instruction::StringConcatenate{target, left, right} => {
                let left = clone_register(*left);
                let right = clone_register(*right);
                match string_concatenate(left, right) {
                    Ok(result) => {
                        set_register(*target, result);
                        program_counter = program_counter.add(1);
                    },
                    Err(error) => {
                        let exception = Value::error_from_error(error);
                        return CallStackDelta::Throw(exception);
                    },
                }
            },

            Instruction::Return{result} => {
                let result = move_register(*result);
                return CallStackDelta::Return(result);
            },

            Instruction::Throw{exception} => {
                let exception = move_register(*exception);
                return CallStackDelta::Throw(exception);
            },

        }

    }
}

/* -------------------------------------------------------------------------- */
/*                              StringConcatenate                             */
/* -------------------------------------------------------------------------- */

#[derive(Debug, Error)]
enum StringConcatenateError
{
    #[error("In left side of `~`: {0}")]
    LeftToString(ToStringError),

    #[error("In right side of `~`: {0}")]
    RightToString(ToStringError),

    #[error("In `~`: {0}")]
    StringFromBytes(#[from] StringFromBytesError),
}

fn string_concatenate(left: Value, right: Value)
    -> Result<Value, StringConcatenateError>
{
    type Error = StringConcatenateError;

    // Convert left and right operands to strings.
    let left  = left .to_string().map_err(Error::LeftToString)?;
    let right = right.to_string().map_err(Error::RightToString)?;

    // Compute length of resulting string.
    let len = usize::checked_add(left.len(), right.len())
        .ok_or(Error::StringFromBytes(StringFromBytesError))?;

    // Create resulting string.
    let bytes = unsafe {
        let mut bytes = Arc::new_uninit_slice(len);
        let borrow = Arc::get_mut_unchecked(&mut bytes);
        MaybeUninit::write_slice(&mut borrow[.. left.len()], &left);
        MaybeUninit::write_slice(&mut borrow[left.len() ..], &right);
        Arc::<[_]>::assume_init(bytes)
    };

    // Wrap resulting string in value.
    Value::string_from_bytes(bytes).map_err(Error::from)
}
