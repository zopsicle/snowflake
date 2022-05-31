use {
    crate::hash::{Hash, hash_file_at_with},
    super::{State, ok_if_already_exists},
    bitflags::bitflags,
    os_ext::{
        S_IFDIR, S_IFLNK, S_IFMT, S_IFREG, S_ISGID, S_ISUID, S_ISVTX,
        RENAME_NOREPLACE,
        renameat2, stat,
    },
    std::{fmt, io, os::unix::io::BorrowedFd, path::Path},
    thiserror::Error,
};

/* -------------------------------------------------------------------------- */
/*                         Cache output implementation                        */
/* -------------------------------------------------------------------------- */

impl State
{
    /// Implementation of [`cache_output`][`Self::cache_output`].
    pub (super) fn cache_output_impl(
        &self,
        dirfd: Option<BorrowedFd>,
        pathname: &Path,
    ) -> Result<Hash, CacheOutputError>
    {
        // Hash the output and check its properties.
        let hash = hash_file_at_with(dirfd, pathname, |statbuf| {
            let error = Self::check_output(statbuf);
            if error.is_empty() {
                Ok(())
            } else {
                Err(io::Error::other(error))
            }
        })?;

        // Move the output to the cache.
        let cache = self.output_cache_dir()?;
        renameat2(
            dirfd,       pathname,
            Some(cache), hash.to_string(),
            RENAME_NOREPLACE,
        ).or_else(ok_if_already_exists)?;

        Ok(hash)
    }

    /// Check that the properties of an output look reasonable.
    fn check_output(&stat{st_mode, st_nlink, ..}: &stat) -> OutputError
    {
        use OutputError as E;
        let file_type = st_mode & S_IFMT;

        let mut err = E::empty();

        // Sketchy stuff that we don't want in the cache.
        if st_mode & S_ISUID != 0 { err |= E::SETUID_BIT; }
        if st_mode & S_ISGID != 0 { err |= E::SETGID_BIT; }
        if st_mode & S_ISVTX != 0 { err |= E::STICKY_BIT; }

        // The set of allowed permissions was chosen conservatively.
        // A future version might allow more combinations of permissions.
        if
            match file_type {
                // NOTE: When changing this, also change Display impl.
                S_IFREG => !matches!(st_mode & 0o777, 0o755 | 0o644),
                S_IFDIR => !matches!(st_mode & 0o777, 0o755),
                S_IFLNK => false,  // Symlinks have no permissions.
                _       => false,  // Bad type handled below.
            }
        {
            err |= E::BAD_PERMISSIONS;
        }

        // The hash function will already error on bad file types,
        // but we can give a better error message through OutputError.
        if !matches!(file_type, S_IFREG | S_IFDIR | S_IFLNK) {
            err |= E::BAD_FILE_TYPE;
        }

        // A file having multiple hard links can cause the output cache
        // to be corrupted by altering the file through another hard link.
        // Directories cannot be hard linked, so ignore those.
        if file_type != S_IFDIR && st_nlink != 1 {
            err |= E::MULTIPLE_HARD_LINKS;
        }

        err
    }
}

/* -------------------------------------------------------------------------- */
/*                             Cache output error                             */
/* -------------------------------------------------------------------------- */

/// Error returned when caching an output.
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum CacheOutputError
{
    #[error("{0}")]
    Io(io::Error),

    #[error("{0}")]
    Output(#[from] OutputError),
}

impl From<io::Error> for CacheOutputError
{
    fn from(mut other: io::Error) -> Self
    {
        let inner: Option<&mut OutputError> =
            other.get_mut().and_then(|err| err.downcast_mut());
        match inner {
            Some(err) => Self::Output(*err),
            None => Self::Io(other),
        }
    }
}

/* -------------------------------------------------------------------------- */
/*                                Output error                                */
/* -------------------------------------------------------------------------- */

bitflags!
{
    /// Error returned when an output has bad properties.
    #[derive(Error)]
    pub struct OutputError: u8
    {
        #[allow(missing_docs)] const SETUID_BIT          = 1 << 0;
        #[allow(missing_docs)] const SETGID_BIT          = 1 << 1;
        #[allow(missing_docs)] const STICKY_BIT          = 1 << 2;
        #[allow(missing_docs)] const BAD_PERMISSIONS     = 1 << 3;
        #[allow(missing_docs)] const BAD_FILE_TYPE       = 1 << 4;
        #[allow(missing_docs)] const MULTIPLE_HARD_LINKS = 1 << 5;
    }
}

impl fmt::Display for OutputError
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
    {
        let c = |f| self.contains(f);
        write!(f, "Output has ")?;
        if c(Self::SETUID_BIT) { write!(f, "the setuid bit set, ")?; }
        if c(Self::SETGID_BIT) { write!(f, "the setgid bit set, ")?; }
        if c(Self::STICKY_BIT) { write!(f, "the sticky bit set, ")?; }
        if c(Self::BAD_PERMISSIONS) {
            write!(f, "regular file permissions other than 755 or 644 or \
                       directory permissions other than 755, ")?;
        }
        if c(Self::BAD_FILE_TYPE) {
            write!(f, "a file type other than regular file, \
                       directory, or symbolic link, ")?;
        }
        if c(Self::MULTIPLE_HARD_LINKS) {
            write!(f, "multiple hard links, ")?;
        }
        write!(f, "and this is not allowed")
    }
}

/* -------------------------------------------------------------------------- */
/*                                    Tests                                   */
/* -------------------------------------------------------------------------- */

#[cfg(test)]
mod tests
{
    use {
        super::*,
        os_ext::{S_IFIFO, S_ISUID, cstr, linkat, mkdirat, mkdtemp, mknodat},
        std::{assert_matches::assert_matches, os::unix::io::AsFd},
    };

    #[test]
    fn bad_files()
    {
        use {CacheOutputError as Coe, OutputError as Oe};

        // Create state directory.
        let path = mkdtemp(cstr!(b"/tmp/snowflake-test-XXXXXX")).unwrap();

        // Create scratch directory.
        let state = State::open(&path).unwrap();
        let scratch = state.new_scratch_dir().unwrap();
        let scratch = Some(scratch.as_fd());

        // Check that a particular file fails to be cached with a given error.
        // This cannot currently be a closure because of [1].
        // [1]: https://github.com/rust-lang/rust/issues/74042
        #[track_caller]
        fn test_case(state: &State, scratch: Option<BorrowedFd>,
                     path: &str, expected: Oe)
        {
            let actual = state.cache_output(scratch, Path::new(path));
            assert_matches!(actual, Err(Coe::Output(err)) if err == expected);
        }

        // Create a bunch of bad files.
        mknodat(scratch, "setuid",  S_IFREG | S_ISUID | 0o644, 0).unwrap();
        mknodat(scratch, "setgid",  S_IFREG | S_ISGID | 0o644, 0).unwrap();
        mknodat(scratch, "sticky",  S_IFREG | S_ISVTX | 0o644, 0).unwrap();
        mknodat(scratch, "regperm", S_IFREG |           0o600, 0).unwrap();
        mkdirat(scratch, "dirperm",                     0o700   ).unwrap();
        mknodat(scratch, "fifo",    S_IFIFO |           0o644, 0).unwrap();
        mknodat(scratch, "link1",   S_IFREG |           0o644, 0).unwrap();
        linkat(scratch, "link1", scratch, "link2", 0).unwrap();

        // Test that caching each file reports the correct error.
        test_case(&state, scratch, "setuid",  Oe::SETUID_BIT);
        test_case(&state, scratch, "setgid",  Oe::SETGID_BIT);
        test_case(&state, scratch, "sticky",  Oe::STICKY_BIT);
        test_case(&state, scratch, "regperm", Oe::BAD_PERMISSIONS);
        test_case(&state, scratch, "dirperm", Oe::BAD_PERMISSIONS);
        test_case(&state, scratch, "fifo",    Oe::BAD_FILE_TYPE);
        test_case(&state, scratch, "link1",   Oe::MULTIPLE_HARD_LINKS);
        test_case(&state, scratch, "link2",   Oe::MULTIPLE_HARD_LINKS);
    }
}
