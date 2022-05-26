use super::{Perform, Result};

extern "C"
{
    fn perform_run_command_gist();
}

pub fn perform_run_command(perform: &Perform) -> Result
{
    unsafe { perform_run_command_gist(); }
    todo!()
}
