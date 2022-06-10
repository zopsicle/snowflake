use {super::*, std::fmt::{Display, Formatter, Result}};

impl Display for ActionLabel
{
    fn fmt(&self, f: &mut Formatter) -> Result
    {
        write!(f, "#{}", self.action)
    }
}

impl Display for ActionOutputLabel
{
    fn fmt(&self, f: &mut Formatter) -> Result
    {
        write!(f, "{}|{}", self.action, self.output)
    }
}
