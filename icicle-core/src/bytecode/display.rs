use {super::{Instruction, Procedure, Register}, std::fmt};

impl<'h> fmt::Display for Procedure<'h>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        for (i, instruction) in self.instructions.iter().enumerate() {
            write!(f, "{i:>6} {instruction}\n")?;
        }
        Ok(())
    }
}

impl fmt::Display for Register
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        write!(f, "R{}", self.0)
    }
}

impl<'h> fmt::Display for Instruction<'h>
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        match self {
            Self::CopyRegister{target, source} =>
                write!(f, "COPY_REGISTER      {target}, {source}"),
            Self::CopyConstant{target, source} =>
                write!(f, "COPY_CONSTANT      {target}, {source:?}"),
            Self::NumericAdd{target, left, right} =>
                write!(f, "NUMERIC_ADD        {target}, {left}, {right}"),
            Self::StringConcatenate{target, left, right} =>
                write!(f, "STRING_CONCATENATE {target}, {left}, {right}"),
            Self::Return{value} =>
                write!(f, "RETURN             {value}"),
        }
    }
}
