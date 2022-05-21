use {crate::bytecode::*, std::fmt::{Display, Formatter, Result}};

struct Lower<'a, T>(&'a T);

impl Display for Lower<'_, Procedure>
{
    fn fmt(&self, f: &mut Formatter) -> Result
    {
        if let Some(max_register) = self.0.max_register {
            for i in 0 ..= max_register.0 {
                write!(f, "local {};", Lower(&Register(i)))?;
            }
        }

        for (i, instruction) in self.0.instructions.iter().enumerate() {
            let i = i.try_into().expect("Too many instructions");
            write!(f, "::{}::;{}", Lower(&Label(i)), Lower(instruction))?;
        }

        Ok(())
    }
}

impl Display for Lower<'_, Register>
{
    fn fmt(&self, f: &mut Formatter) -> Result
    {
        write!(f, "r{}", self.0.0)
    }
}

impl Display for Lower<'_, Label>
{
    fn fmt(&self, f: &mut Formatter) -> Result
    {
        write!(f, "l{}", self.0.0)
    }
}

impl Display for Lower<'_, Instruction>
{
    fn fmt(&self, f: &mut Formatter) -> Result
    {
        match self.0 {
            Instruction::Copy{target, source} =>
                write!(f, "{} = {};", Lower(target), Lower(source)),
            Instruction::Jump{target} =>
                write!(f, "goto {};", Lower(target)),
            Instruction::JumpIf{target, condition} =>
                write!(f, "if sekka_to_boolean({}) then goto {} end;",
                    Lower(condition), Lower(target)),
            Instruction::Return{value} =>
                // Return cannot be followed by a label,
                // so we need to wrap it in a do–end block.
                write!(f, "do return {} end;", Lower(value)),
            Instruction::ToBoolean{target, operand} =>
                write!(f, "{} = sekka_to_boolean({});",
                    Lower(target), Lower(operand)),
            Instruction::ToNumeric{target, operand} =>
                write!(f, "{} = sekka_to_numeric({});",
                    Lower(target), Lower(operand)),
            Instruction::ToString{target, operand} =>
                write!(f, "{} = sekka_to_string({});",
                    Lower(target), Lower(operand)),
        }
    }
}
