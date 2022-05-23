use {
    crate::{
        bytecode::{Instruction, Register},
        string_from_format,
        value::{StringFromBytesError, Value},
    },
    std::{mem::{MaybeUninit, replace}, sync::Arc},
};

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
pub unsafe fn interpret(
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
                    Err(exception) =>
                        return CallStackDelta::Throw(exception),
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

fn string_concatenate(left: Value, right: Value) -> Result<Value, Value>
{
    // Convert left and right operands to strings.
    let mkerr = |side, err| string_from_format!("In {side} side of `~`: {err}");
    let left  = left .to_string().map_err(|err| mkerr("left" , err))?;
    let right = right.to_string().map_err(|err| mkerr("right", err))?;

    // Compute length of resulting string.
    let len = usize::checked_add(left.len(), right.len())
        .ok_or_else(|| string_from_format!("In `~`: {StringFromBytesError}"))?;

    // Create resulting string.
    let bytes = unsafe {
        let mut bytes = Arc::new_uninit_slice(len);
        let borrow = Arc::get_mut_unchecked(&mut bytes);
        MaybeUninit::write_slice(&mut borrow[.. left.len()], &left);
        MaybeUninit::write_slice(&mut borrow[left.len() ..], &right);
        Arc::<[_]>::assume_init(bytes)
    };

    // Wrap resulting string in value.
    Value::string_from_bytes(bytes)
        .map_err(|err| string_from_format!("In `~`: {err}"))
}
