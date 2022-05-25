use {
    super::{Perform, Result, Summary},
    os_ext::{O_CREAT, O_WRONLY, openat},
    std::{fs::File, io::Write, path::Path},
};

pub fn perform_write_regular_file(
    perform: &Perform,
    content: &[u8],
    executable: bool,
) -> Result
{
    let flags = O_CREAT | O_WRONLY;
    let mode = if executable { 0o755 } else { 0o644 };
    let file = openat(Some(perform.outputs), Path::new("0"), flags, mode)?;
    File::from(file).write_all(content)?;
    Ok(Summary{warnings: false})
}
