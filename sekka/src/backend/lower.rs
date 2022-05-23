use {crate::ir::{Instruction, Register, Value}, std::fmt::{Result, Write}};

/// Lower an IR register to JavaScript code.
pub fn lower_register(w: &mut dyn Write, register: Register) -> Result
{
    write!(w, "r{}", register.0)
}

/// Lower an IR value to JavaScript code.
pub fn lower_value(w: &mut dyn Write, value: &Value) -> Result
{
    match value {

        Value::Register(register) =>
            lower_register(w, *register),

        Value::String(value) => {
            write!(w, "Uint8Array.of(")?;
            for (i, byte) in value.iter().enumerate() {
                if i > 0 {
                    write!(w, ", ")?;
                }
                write!(w, "{}", byte)?;
            }
            write!(w, ")")
        },

    }
}

/// Lower an IR instruction to JavaScript code.
pub fn lower_instruction(
    w: &mut dyn Write,
    register: Register,
    instruction: &Instruction,
) -> Result
{
    match instruction {

        Instruction::NumericAdd{left, right} => {
            write!(w, "var ")?;
            lower_register(w, register)?;
            write!(w, " = sekka_numeric_add(")?;
            lower_value(w, left)?;
            write!(w, ", ")?;
            lower_value(w, right)?;
            write!(w, ");\n")
        },

        Instruction::StringConcatenate{left, right} => {
            write!(w, "var ")?;
            lower_register(w, register)?;
            write!(w, " = sekka_string_concatenate(")?;
            lower_value(w, left)?;
            write!(w, ", ")?;
            lower_value(w, right)?;
            write!(w, ");\n")
        },

        Instruction::Return{result} => {
            write!(w, "return ")?;
            lower_value(w, result)?;
            write!(w, ";\n")
        },

    }
}
