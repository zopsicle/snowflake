//! Working with state directories.

pub use self::cache_output::*;

use {
    os_ext::{
        AT_SYMLINK_FOLLOW,
        O_DIRECTORY, O_PATH, O_RDONLY, O_TMPFILE, O_WRONLY,
        cstr, linkat, mkdirat, open, openat,
        io::magic_link,
    },
    serde::{Deserialize, Serialize},
    snowflake_util::hash::Hash,
    std::{
        ffi::{CStr, CString},
        fs::File,
        io::{self, BufReader, ErrorKind::{AlreadyExists, NotFound}, Write},
        lazy::SyncOnceCell,
        os::unix::io::{AsFd, BorrowedFd, OwnedFd},
        sync::atomic::{AtomicU32, Ordering::SeqCst},
    },
};

mod cache_output;

// Paths to the different components of the state directory.
// TODO: Replace with cstr! macro once from_ptr is const.
const SCRATCHES_DIR: &CStr =
    unsafe { CStr::from_bytes_with_nul_unchecked(b"scratches\0") };
const ACTION_CACHE_DIR: &CStr =
    unsafe { CStr::from_bytes_with_nul_unchecked(b"action-cache\0") };
const OUTPUT_CACHE_DIR: &CStr =
    unsafe { CStr::from_bytes_with_nul_unchecked(b"output-cache\0") };

/// Handle to a state directory.
pub struct State
{
    /// Handle to the state directory.
    state_dir: OwnedFd,

    // Handles to the different components of the state directory.
    scratches_dir:    SyncOnceCell<OwnedFd>,
    action_cache_dir: SyncOnceCell<OwnedFd>,
    output_cache_dir: SyncOnceCell<OwnedFd>,

    /// Name of the next scratch file to create.
    next_scratch: AtomicU32,
}

/// Cached information about an action.
#[derive(Debug, Deserialize, Serialize)]
pub struct ActionCacheEntry
{
    /// The hash of the build log.
    ///
    /// This enables finding the build log in the output cache.
    pub build_log: Hash,

    /// The hash of each output of the action.
    ///
    /// The number of outputs must equal [`Action::outputs`]
    /// and their indices must match those in [output labels].
    ///
    /// [`Action::outputs`]: `crate::action::Action::outputs`
    /// [output labels]: `crate::label::ActionOutputLabel`
    pub outputs: Vec<Hash>,

    /// Whether warnings were emitted by the action.
    ///
    /// See the manual entry on warnings for
    /// the implications of setting this flag.
    pub warnings: bool,
}

impl State
{
    /// Open a state directory.
    ///
    /// The state directory must already exist.
    /// Components of the state directory are not opened immediately;
    /// they are opened when they are first used.
    pub fn open(path: &CStr) -> io::Result<Self>
    {
        let state_dir = open(path, O_DIRECTORY | O_PATH, 0)?;

        let this = Self{
            state_dir,
            scratches_dir:    SyncOnceCell::new(),
            action_cache_dir: SyncOnceCell::new(),
            output_cache_dir: SyncOnceCell::new(),
            next_scratch:     AtomicU32::new(0),
        };

        Ok(this)
    }

    /// Handle to the scratches directory.
    ///
    /// The scratches directory contains scratch files.
    /// A scratch file is a temporary file for use while building.
    fn scratches_dir(&self) -> io::Result<BorrowedFd>
    {
        self.ensure_open_dir_once(&self.scratches_dir, SCRATCHES_DIR)
    }

    /// Create and open a new scratch directory.
    ///
    /// The scratch directory starts out empty.
    pub fn new_scratch_dir(&self) -> io::Result<OwnedFd>
    {
        let scratches_dir = self.scratches_dir()?;
        let scratch_id = self.next_scratch.fetch_add(1, SeqCst);
        let path = CString::new(scratch_id.to_string()).unwrap();
        mkdirat(Some(scratches_dir), &path, 0o755)?;
        openat(Some(scratches_dir), &path, O_DIRECTORY | O_PATH, 0)
    }

    /// Link a file in the scratches directory.
    ///
    /// Returns the file descriptor for the scratches directory
    /// and the relative path to the newly created link.
    pub fn new_scratch_link(&self, fd: BorrowedFd)
        -> io::Result<(BorrowedFd, CString)>
    {
        let scratches_dir = self.scratches_dir()?;
        let scratch_id = self.next_scratch.fetch_add(1, SeqCst);
        let path = CString::new(scratch_id.to_string()).unwrap();
        linkat(
            None, &magic_link(fd),
            Some(scratches_dir), &path,
            AT_SYMLINK_FOLLOW,
        )?;
        Ok((scratches_dir, path))
    }

    /// Handle to the action cache.
    fn action_cache_dir(&self) -> io::Result<BorrowedFd>
    {
        self.ensure_open_dir_once(&self.action_cache_dir, ACTION_CACHE_DIR)
    }

    /// Insert an entry into the action cache.
    ///
    /// The entry is stored at the given action hash.
    /// If the entry already exists, nothing is changed.
    pub fn cache_action(&self, hash: Hash, entry: &ActionCacheEntry)
        -> io::Result<()>
    {
        let cache = self.action_cache_dir()?;

        // Open a file to store the cache entry.
        let flags = O_TMPFILE | O_WRONLY;
        let file = openat(Some(cache), cstr!(b"."), flags, 0o644)?;

        // Write the cache entry to a file.
        let mut file = File::from(file);
        serde_json::to_writer(&mut file, entry)?;
        file.flush()?;

        // Create the file in the action cache.
        linkat(
            None, &magic_link(file.as_fd()),
            Some(cache), &CString::new(hash.to_string()).unwrap(),
            AT_SYMLINK_FOLLOW,
        ).or_else(ok_if_already_exists)?;

        Ok(())
    }

    /// Read an entry from the action cache.
    ///
    /// If there is no entry for the given action,
    /// this method returns [`None`].
    pub fn cached_action(&self, hash: Hash)
        -> io::Result<Option<ActionCacheEntry>>
    {
        let cache = self.action_cache_dir()?;
        let pathname = &CString::new(hash.to_string()).unwrap();
        match openat(Some(cache), pathname, O_RDONLY, 0) {
            Ok(file) => {
                let file = File::from(file);
                let file = BufReader::new(file);
                let entry = serde_json::from_reader(file)?;
                Ok(Some(entry: ActionCacheEntry))
            },
            Err(err) if err.kind() == NotFound => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Handle to the output cache.
    fn output_cache_dir(&self) -> io::Result<BorrowedFd>
    {
        self.ensure_open_dir_once(&self.output_cache_dir, OUTPUT_CACHE_DIR)
    }

    /// Move a file to the output cache.
    ///
    /// This method computes the hash of the file
    /// and checks that it qualifies for caching.
    /// Then it renames the file so it is in the cache.
    /// If an equivalent file was already cached, the file is not renamed.
    pub fn cache_output(&self, dirfd: Option<BorrowedFd>, pathname: &CStr)
        -> Result<Hash, CacheOutputError>
    {
        self.cache_output_impl(dirfd, pathname)
    }

    /// Insert a build log into the output cache.
    ///
    /// Build logs are opened with `O_TMPFILE`, so they don't have a path.
    /// They cannot be inserted using [`cache_output`][`Self::cache_output`].
    /// This method first creates a scratch link, then moves it to the cache.
    /// This method takes ownership of and closes the build log,
    /// because it must not be modified after adding it to the cache.
    pub fn cache_build_log(&self, build_log: OwnedFd)
        -> io::Result<Hash>
    {
        let (scratches_dir, build_log_path) =
            self.new_scratch_link(build_log.as_fd())?;

        drop(build_log);

        match self.cache_output(Some(scratches_dir), &build_log_path) {
            Ok(hash) => Ok(hash),
            Err(CacheOutputError::Io(err)) => Err(err),
            Err(CacheOutputError::Output(err)) =>
                panic!("Build logs should always qualify for caching: {err}"),
        }
    }

    /// Obtain the path to a cached output.
    ///
    /// Returns the file descriptor for the output cache
    /// and the relative path to the cached output.
    ///
    /// If there is no such output, this method still succeeds.
    /// The returned path would point to a non-existing file.
    /// Since the output cache is content-addressed,
    /// this would mean there is a dangling reference somewhere.
    /// The caller should interpret this as a bug and crash.
    pub fn cached_output(&self, hash: Hash)
        -> io::Result<(BorrowedFd, CString)>
    {
        let dirfd = self.output_cache_dir()?;
        let path = CString::new(hash.to_string()).unwrap();
        Ok((dirfd, path))
    }

    /// Ensure that a directory exists and open it.
    fn ensure_open_dir_once<'a>(
        &self,
        cell: &'a SyncOnceCell<OwnedFd>,
        path: &CStr,
    ) -> io::Result<BorrowedFd<'a>>
    {
        let owned_fd = cell.get_or_try_init(|| {
            let dirfd = Some(self.state_dir.as_fd());
            mkdirat(dirfd, path, 0o755)
                .or_else(ok_if_already_exists)?;
            openat(dirfd, path, O_DIRECTORY | O_PATH, 0)
        })?;
        Ok(owned_fd.as_fd())
    }
}

impl AsFd for State
{
    fn as_fd(&self) -> BorrowedFd
    {
        self.state_dir.as_fd()
    }
}

fn hash_to_path(hash: &Hash) -> CString
{
    CString::new(hash.to_string())
        .expect("Hash as Display should not write nul")
}

fn ok_if_already_exists(err: io::Error) -> io::Result<()>
{
    if err.kind() == AlreadyExists {
        Ok(())
    } else {
        Err(err)
    }
}

#[cfg(test)]
mod tests
{
    use {
        super::*,
        os_ext::{
            O_CREAT, O_WRONLY,
            cstr, cstring, mkdtemp, readlink,
            cstr::CStrExt,
        },
        std::{os::unix::io::AsFd},
    };

    #[test]
    fn new_scratch_dir()
    {
        // Create state directory.
        let path = mkdtemp(cstring!(b"/tmp/snowflake-test-XXXXXX")).unwrap();

        // Create two scratch directories.
        let state = State::open(&path).unwrap();
        let scratch_dir_0 = state.new_scratch_dir().unwrap();
        let scratch_dir_1 = state.new_scratch_dir().unwrap();

        // Test paths to the scratch directories.
        let magic_link_0 = magic_link(scratch_dir_0.as_fd());
        let magic_link_1 = magic_link(scratch_dir_1.as_fd());
        let scratch_dir_path_0 = readlink(&magic_link_0).unwrap();
        let scratch_dir_path_1 = readlink(&magic_link_1).unwrap();
        assert_eq!(scratch_dir_path_0, path.join(cstr!(b"scratches/0")));
        assert_eq!(scratch_dir_path_1, path.join(cstr!(b"scratches/1")));

        // Test that scratch directory is writable.
        openat(
            Some(scratch_dir_0.as_fd()),
            cstr!(b"build.log"),
            O_CREAT | O_WRONLY,
            0o644,
        ).unwrap();
    }

    #[test]
    fn action_cache()
    {
        // Create state directory.
        let path = mkdtemp(cstring!(b"/tmp/snowflake-test-XXXXXX")).unwrap();
        let state = State::open(&path).unwrap();

        // Prepare action for inserting into action cache.
        let hash = Hash([0; 32]);
        let entry = ActionCacheEntry{
            build_log: Hash([1; 32]),
            outputs: vec![Hash([2; 32]), Hash([3; 32])],
            warnings: true,
        };

        // Insert action into cache and retrieve from cache.
        state.cache_action(hash, &entry).unwrap();
        let retrieved = state.cached_action(hash).unwrap().unwrap();

        // Check that the entry was retrieved correctly.
        assert_eq!(format!("{entry:?}"), format!("{retrieved:?}"));

        // Retrieving a non-existent action should return None.
        assert!(state.cached_action(Hash([4; 32])).unwrap().is_none());
    }
}
