use {
    anyhow::Context,
    os_ext::{O_CREAT, O_WRONLY, openat},
    snowflake_core::action::{Action, Outputs, Perform, Result, Success},
    snowflake_util::hash::{Blake3, Hash},
    std::{fs::File, io::Write, path::PathBuf},
};

/// Action that writes a regular file.
pub struct WriteRegularFile
{
    /// The content of the regular file.
    pub content: Vec<u8>,

    /// Whether the executable bit is set
    /// in the mode of the regular file.
    pub executable: bool,
}

impl Action for WriteRegularFile
{
    fn inputs(&self) -> usize
    {
        0
    }

    fn outputs(&self) -> Outputs<usize>
    {
        Outputs::Outputs(1)
    }

    fn perform(&self, perform: &Perform, input_paths: &[PathBuf]) -> Result
    {
        debug_assert_eq!(input_paths.len(), 0);
        let output_path = PathBuf::from("output");
        let flags = O_CREAT | O_WRONLY;
        let mode = if self.executable { 0o755 } else { 0o644 };
        let file = openat(Some(perform.scratch), &output_path, flags, mode)
            .context("Open regular file")?;
        File::from(file).write_all(&self.content)
            .context("Write regular file")?;
        Ok(Success{output_paths: vec![output_path], warnings: false})
    }

    fn hash(&self, input_hashes: &[Hash]) -> Hash
    {
        // NOTE: See the manual chapter on avoiding hash collisions.

        let Self{content, executable} = self;

        debug_assert_eq!(input_hashes.len(), 0);

        let mut h = Blake3::new();
        h.put_str("WriteRegularFile");
        h.put_bytes(content);
        h.put_bool(*executable);
        h.finalize()
    }
}
