use {
    super::{Blake3, Hash},
    os_ext::{
        AT_SYMLINK_NOFOLLOW,
        O_DIRECTORY, O_NOFOLLOW, O_RDONLY,
        S_IFDIR, S_IFLNK, S_IFMT, S_IFREG, S_IXUSR,
        fdopendir,
        fstatat,
        openat,
        readdir,
        readlinkat,
        stat,
    },
    std::{
        fs::File,
        io::{self, Write, copy},
        os::unix::{ffi::OsStrExt, io::{AsFd, BorrowedFd}},
        path::Path,
    },
};

/// Hash a file at a given path.
///
/// The file may either be a regular file, a symbolic link,
/// or a directory containing only such eligible files.
/// If a file is encountered that is inaccessible or of unsupported type
/// this function returns an error and the file cannot be hashed.
///
/// # Contents of the hash
///
/// The path of the file is not included in the hash.
/// That is, `hash_file_at(dirfd, "foo")` and `hash_file_at(dirfd, "bar")`
/// will return the same hash if the files are otherwise the same.
///
/// If the file is a regular file, the hash contains
/// its contents and whether it is executable.
/// If the file is a directory, the hash contains
/// recursively the entries of the directory,
/// including the names of the entries.
/// If the file is a symbolic link, the hash contains
/// the target name of the symbolic link (it is not followed).
///
/// Inode, mode, owner, dates, etc are not included in the hash.
/// They are assumed to be uninteresting to any actions or artifact consumers.
/// The read permission bit is ignored as unreadable files cannot be hashed.
/// The write permission bit is ignored as inputs are mounted read-only,
/// and outputs are made read-only before being added to the output cache.
/// The execute permission bit is ignored for directories
/// as non-executable directories cannot be hashed.
pub fn hash_file_at(dirfd: Option<BorrowedFd>, path: impl AsRef<Path>)
    -> io::Result<Hash>
{
    hash_file_at_with(dirfd, path, |_| Ok(()))
}

/// Like [`hash_file_at`], but with customizable extra checks.
///
/// Hashing a file already stats every file to be hashed,
/// so if you're interested in those statbufs,
/// this function will call `f` for each of them.
pub fn hash_file_at_with(
    dirfd: Option<BorrowedFd>,
    path:  impl AsRef<Path>,
    mut f: impl FnMut(&stat) -> io::Result<()>,
) -> io::Result<Hash>
{
    let mut blake3 = Blake3::new();
    write_file_at(&mut blake3, dirfd, path.as_ref(), &mut f)?;
    Ok(blake3.finalize())
}

// NOTE: See the manual chapter on avoiding hash collisions.

fn write_file_at(
    writer: &mut impl Write,
    dirfd:  Option<BorrowedFd>,
    path:   &Path,
    f:      &mut dyn FnMut(&stat) -> io::Result<()>
) -> io::Result<()>
{
    let statbuf = fstatat(dirfd, path, AT_SYMLINK_NOFOLLOW)?;
    f(&statbuf)?;
    match statbuf.st_mode & S_IFMT {
        S_IFREG => write_reg_at(writer, dirfd, path, &statbuf),
        S_IFDIR => write_dir_at(writer, dirfd, path, f),
        S_IFLNK => write_lnk_at(writer, dirfd, path),
        _       => todo!("Return error about unsupported file type"),
    }
}

// Byte which indicates the type of file.
const FILE_TYPE_REG: u8 = 0;
const FILE_TYPE_DIR: u8 = 1;
const FILE_TYPE_LNK: u8 = 2;

/// Write a regular file.
fn write_reg_at(
    writer:  &mut impl Write,
    dirfd:   Option<BorrowedFd>,
    path:    &Path,
    statbuf: &stat,
) -> io::Result<()>
{
    // Write file type.
    writer.write_all(&[FILE_TYPE_REG])?;

    // Write whether file is executable.
    let executable = statbuf.st_mode & S_IXUSR != 0;
    writer.write_all(&[executable as u8])?;

    // Write file size.
    writer.write_all(&(statbuf.st_size as u64).to_le_bytes())?;

    // Write file contents.
    let file = openat(dirfd, path, O_NOFOLLOW | O_RDONLY, 0)?;
    let mut file = File::from(file);
    copy(&mut file, writer)?;

    Ok(())
}

/// Write a directory.
fn write_dir_at(
    writer: &mut impl Write,
    dirfd:  Option<BorrowedFd>,
    path:   &Path,
    f:      &mut dyn FnMut(&stat) -> io::Result<()>
) -> io::Result<()>
{
    // Write directory metadata.
    writer.write_all(&[FILE_TYPE_DIR])?;

    // Open directory for reading.
    let dir = openat(dirfd, path, O_DIRECTORY | O_NOFOLLOW | O_RDONLY, 0)?;

    // Collect directory entries.
    let mut stream = fdopendir(dir.try_clone()?)?;
    let mut entries = Vec::new();
    while let Some(dirent) = readdir(&mut stream)? {
        let d_name = dirent.d_name;
        if d_name != Path::new(".") && d_name != Path::new("..") {
            entries.push(d_name);
        }
    }
    drop(stream);

    // Make sure the order is always the same.
    entries.sort();

    // Recursively write the entries.
    for entry in entries {

        // Write entry name.
        writer.write_all(entry.as_os_str().as_bytes())?;
        writer.write_all(&[0])?;

        // Recursively write entry.
        write_file_at(writer, Some(dir.as_fd()), &entry, f)?;

    }

    // Write directory entries terminator.
    // Pathnames cannot be empty, so this is unambiguous.
    writer.write_all(&[0])
}

/// Write a symbolic link.
fn write_lnk_at(
    writer:  &mut impl Write,
    dirfd:   Option<BorrowedFd>,
    path:    &Path,
) -> io::Result<()>
{
    // Write symbolic link metadata.
    writer.write_all(&[FILE_TYPE_LNK])?;

    // Do not write file permissions.
    // Symbolic links don't have those.
    { }

    // Write link target.
    let target = readlinkat(dirfd, path)?;
    writer.write_all(target.as_bytes_with_nul())
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn example()
    {
        let expected = &[
            1,
                b'b', b'r', b'o', b'k', b'e', b'n', b'.', b'l', b'n', b'k', 0,
                    2, b'e', b'n', b'o', b'e', b'n', b't', b'.', b't', b'x', b't', 0,
                b'd', b'i', b'r', b'e', b'c', b't', b'o', b'r', b'y', 0,
                    1,
                        b'b', b'a', b'r', b'.', b't', b'x', b't', 0,
                            0, 1,
                                4, 0, 0, 0, 0, 0, 0, 0,
                                b'b', b'a', b'r', b'\n',
                        b'f', b'o', b'o', b'.', b't', b'x', b't', 0,
                            0, 0,
                                4, 0, 0, 0, 0, 0, 0, 0,
                                b'f', b'o', b'o', b'\n',
                        0,
                b'r', b'e', b'g', b'u', b'l', b'a', b'r', b'.', b't', b'x', b't', 0,
                    0, 0,
                        14, 0, 0, 0, 0, 0, 0, 0,
                        b'H', b'e', b'l', b'l', b'o', b',', b' ',
                        b'w', b'o', b'r', b'l', b'd', b'!', b'\n',
                b's', b'y', b'm', b'l', b'i', b'n', b'k', b'.', b'l', b'n', b'k', 0,
                    2, b'r', b'e', b'g', b'u', b'l', b'a', b'r', b'.', b't', b'x', b't', 0,
                0,
        ];

        let expected_hash = Blake3::new().update(expected).finalize();

        let path = Path::new("testdata/hash_file_at");

        let mut buf = Vec::new();
        write_file_at(&mut buf, None, path, &mut |_| Ok(())).unwrap();
        assert_eq!(buf, expected);

        let hash = hash_file_at(None, path).unwrap();
        assert_eq!(hash, expected_hash);
    }
}
