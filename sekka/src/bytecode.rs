pub struct Procedure
{
    pub max_register: Option<Register>,
    pub instructions: Vec<Instruction>,
}

#[derive(Clone, Copy)]
pub struct Register(pub u16);

#[derive(Clone, Copy)]
pub struct Label(pub u16);

pub enum Instruction
{
    Copy{target: Register, source: Register},
    Jump{target: Label},
    JumpIf{target: Label, condition: Register},
    Return{value: Register},
    ToBoolean{target: Register, operand: Register},
    ToNumeric{target: Register, operand: Register},
    ToString{target: Register, operand: Register},
}
