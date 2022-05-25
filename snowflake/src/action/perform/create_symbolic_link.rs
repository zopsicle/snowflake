use {
    super::{Perform, Result, Summary},
    os_ext::symlinkat,
    std::{ffi::CStr, path::Path},
};

pub fn perform_create_symbolic_link(perform: &Perform, target: &CStr) -> Result
{
    symlinkat(target, Some(perform.outputs), Path::new("0"))?;
    Ok(Summary{warnings: false})
}
