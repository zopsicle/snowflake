use {
    super::{Blake3, Hash},
    os_ext::{
        AT_SYMLINK_NOFOLLOW,
        O_DIRECTORY, O_NOFOLLOW, O_RDONLY,
        S_IFDIR, S_IFLNK, S_IFMT, S_IFREG,
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
/// For regular files, the hash contains the permission bits and contents.
/// For directories, the hash contains the permission bits and entries.
/// This function descends into directories; its entries are also hashed.
/// For symbolic links, the hash contains the target.
///
/// If a file is encountered that is inaccessible or of unsupported type
/// this function returns an error and the file cannot be hashed.
/// Note that the permission bits must at least permit the caller
/// to open the file (for regular files) or list the entries (for directories).
pub fn hash_file_at<P>(dirfd: Option<BorrowedFd>, path: P)
    -> io::Result<Hash>
    where P: AsRef<Path>
{
    let mut blake3 = Blake3::new();
    write_file_at(&mut blake3, dirfd, path.as_ref())?;
    Ok(blake3.finalize())
}

// In the below code, we write a file to a writer (usually a Blake3).
// It is very important that different files are hashed differently;
// otherwise it would be possible to create a corrupted build cache!
// An easy way to ensure this is to write the data in such a way
// that it can theoretically be parsed to reconstruct the original file.
// To make sure there are no disambiguities involving variable-length data,
// we either prefix such data with a number indicating their length,
// or we terminate it with a suitable sentinel value.

fn write_file_at(
    writer: &mut impl Write,
    dirfd:  Option<BorrowedFd>,
    path:   &Path,
) -> io::Result<()>
{
    let statbuf = fstatat(dirfd, path, AT_SYMLINK_NOFOLLOW)?;
    match statbuf.st_mode & S_IFMT {
        S_IFREG => write_reg_at(writer, dirfd, path, &statbuf),
        S_IFDIR => write_dir_at(writer, dirfd, path, &statbuf),
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

    // Write file permissions.
    writer.write_all(&(statbuf.st_mode as u16 & 0o777).to_le_bytes())?;

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
    writer:  &mut impl Write,
    dirfd:   Option<BorrowedFd>,
    path:    &Path,
    statbuf: &stat,
) -> io::Result<()>
{
    // Write directory metadata.
    writer.write_all(&[FILE_TYPE_DIR])?;

    // Write file permissions.
    writer.write_all(&(statbuf.st_mode as u16 & 0o777).to_le_bytes())?;

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
        write_file_at(writer, Some(dir.as_fd()), &entry)?;

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
    writer.write_all(target.as_os_str().as_bytes())?;

    // Write link target terminator.
    // Link targets do not contain NULs.
    writer.write_all(&[0])
}

#[cfg(test)]
mod tests
{
    use super::*;

    #[test]
    fn example()
    {
        let expected = &[
            1, 237, 1,
                b'b', b'r', b'o', b'k', b'e', b'n', b'.', b'l', b'n', b'k', 0,
                    2, b'e', b'n', b'o', b'e', b'n', b't', b'.', b't', b'x', b't', 0,
                b'd', b'i', b'r', b'e', b'c', b't', b'o', b'r', b'y', 0,
                    1, 237, 1,
                        b'b', b'a', b'r', b'.', b't', b'x', b't', 0,
                            0, 164, 1,
                                4, 0, 0, 0, 0, 0, 0, 0,
                                b'b', b'a', b'r', b'\n',
                        b'f', b'o', b'o', b'.', b't', b'x', b't', 0,
                            0, 164, 1,
                                4, 0, 0, 0, 0, 0, 0, 0,
                                b'f', b'o', b'o', b'\n',
                        0,
                b'r', b'e', b'g', b'u', b'l', b'a', b'r', b'.', b't', b'x', b't', 0,
                    0, 164, 1,
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
        write_file_at(&mut buf, None, path).unwrap();
        assert_eq!(buf, expected);

        let hash = hash_file_at(None, path).unwrap();
        assert_eq!(hash, expected_hash);
    }
}
