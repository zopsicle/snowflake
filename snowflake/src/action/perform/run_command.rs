use {
    crate::basename::Basename,
    super::{Error, Perform, Summary},
    os_ext::{
        cstr,
        getgid,
        getuid,
        mkdirat,
        mknodat,
        pipe2,
        readlink,
        symlinkat,
    },
    scope_exit::ScopeExit,
    std::{
        ffi::{CStr, CString},
        fs::File,
        io::{self, Read},
        mem::{forget, size_of_val, zeroed},
        os::unix::{
            ffi::OsStrExt,
            io::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
            process::ExitStatusExt,
        },
        panic::always_abort,
        path::Path,
        process::ExitStatus,
        ptr::{addr_of, addr_of_mut, null, null_mut},
        sync::Arc,
        time::Duration,
    },
};

pub fn perform_run_command(
    perform: &Perform,
    outputs: &[Arc<Basename>],
    program: &Path,
    arguments: &[CString],
    environment: &[CString],
    timeout: Duration,
) -> Result<Summary, Error>
{
    // Perform the gist of the performing.
    gist(perform.log, perform.scratch, program,
         arguments, environment, timeout)?;

    // Collect all the output paths.
    let build_dir = Path::new("build");
    let outputs = outputs.iter().map(|b| build_dir.join(&**b)).collect();

    Ok(Summary{outputs, warnings: false})
}

fn gist(
    log: BorrowedFd,
    scratch: BorrowedFd,
    program: &Path,
    arguments: &[CString],
    environment: &[CString],
    timeout: Duration,
) -> Result<(), Error>
{
    // Prepared mounts, with targets relative to the scratch directory.
    // This vec will be appended onto at various points in this function.
    let mut mounts = Vec::new();

    // Set up directory hierarchy in scratch directory.
    // The scratch directory will be chrooted into by the child.
    mkdirat(Some(scratch), cstr!(b"bin"),       0o755)?;
    mkdirat(Some(scratch), cstr!(b"dev"),       0o755)?;
    mkdirat(Some(scratch), cstr!(b"nix"),       0o755)?;
    mkdirat(Some(scratch), cstr!(b"nix/store"), 0o755)?;
    mkdirat(Some(scratch), cstr!(b"proc"),      0o555)?;  // (sic)
    mkdirat(Some(scratch), cstr!(b"root"),      0o755)?;
    mkdirat(Some(scratch), cstr!(b"usr"),       0o755)?;
    mkdirat(Some(scratch), cstr!(b"usr/bin"),   0o755)?;
    mkdirat(Some(scratch), cstr!(b"build"),     0o755)?;

    // Create symbolic links for the standard streams.
    symlinkat(cstr!(b"/proc/self/fd/0"), Some(scratch), cstr!(b"dev/stdin"))?;
    symlinkat(cstr!(b"/proc/self/fd/1"), Some(scratch), cstr!(b"dev/stdout"))?;
    symlinkat(cstr!(b"/proc/self/fd/2"), Some(scratch), cstr!(b"dev/stderr"))?;

    // Prepare writes to /proc/self/gid_map and /proc/self/uid_map.
    // These files map users and groups inside the container
    // to users and groups outside the container.
    let setgroups = "deny\n";
    let uid_map = format!("0 {} 1\n", getuid());
    let gid_map = format!("0 {} 1\n", getgid());

    // For some unknown reason, fchdir(scratch) keeps mount from working.
    // But with chdir(scratch_path) it works, so we do that instead.
    let scratch_magic = format!("/proc/self/fd/{}", scratch.as_raw_fd());
    let scratch_path: CString = readlink(scratch_magic)?;

    // systemd mounts / as MS_SHARED, but MS_PRIVATE is more isolated.
    let root_flags = libc::MS_PRIVATE | libc::MS_REC;
    PreparedMount::new(&mut mounts, "none", "/", "", root_flags, "");

    // Mount the /proc file system which many programs don't work without.
    let proc_flags = libc::MS_NODEV | libc::MS_NOEXEC | libc::MS_NOSUID;
    PreparedMount::new(&mut mounts, "proc", "proc", "proc", proc_flags, "");

    // Mount the Nix store.
    PreparedMount::new_rdonly_bind(&mut mounts, "/nix/store", "nix/store");

    // Mount the device files onto fresh regular files.
    // We need to mount them, because creating device files directly
    // fails with EPERM, even when mknod is called inside the container.
    for path in ["null", "zero", "full", "random", "urandom"] {
        let absolute = format!("/dev/{}", path);
        let relative = format!("dev/{}", path);
        let flags = libc::MS_BIND | libc::MS_REC;
        mknodat(Some(scratch), &relative, 0, 0)?;
        PreparedMount::new(&mut mounts, &absolute, &relative, "", flags, "");
    }

    // Prepare arguments to execve.
    let program = CString::new(program.as_os_str().as_bytes())?;
    let (execve_argv, _execve_argv) = prepare_argv_envp(arguments);
    let (execve_envp, _execve_envp) = prepare_argv_envp(environment);

    // This pipe is used by the child to send pre-execve errors to the parent.
    // Since CLOEXEC is enabled, the parent knows execve has succeeded.
    let (pipe_r, pipe_w) = pipe2(0)?;

    // Zero-initialize this because we don't use most of its features.
    let mut cl_args = unsafe { zeroed::<clone_args>() };

    // Enable all the namespace features.
    cl_args.flags |= (
        libc::CLONE_NEWCGROUP |  // New cgroup namespace.
        libc::CLONE_NEWIPC    |  // New IPC namespace.
        libc::CLONE_NEWNET    |  // New network namespace.
        libc::CLONE_NEWNS     |  // New mount namespace.
        libc::CLONE_NEWPID    |  // New PID namespace.
        libc::CLONE_NEWUSER   |  // New user namespace.
        libc::CLONE_NEWUTS       // New UTS namespace.
    ) as u64;

    // Atomically create a pidfd for use with ppoll.
    // The pidfd will have CLOEXEC enabled, yay!
    let mut pidfd = -1;
    cl_args.flags |= libc::CLONE_PIDFD as u64;
    cl_args.pidfd = addr_of_mut!(pidfd) as u64;

    // We don't actually care about the exit signal,
    // but if we don't set this then waitpid doesn't work.
    cl_args.exit_signal = libc::SIGCHLD as u64;

    // Spawn the child process using the clone3 system call.
    // The interface is similar to that of the fork system call:
    // 0 is returned in the child, pid is returned in the parent.
    let pid = unsafe {
        libc::syscall(
            libc::SYS_clone3,
            // syscall is variadic so let's be explicit about types.
            addr_of!(cl_args): *const clone_args,
            size_of_val(&cl_args): libc::size_t,
        )
    };

    // NOTE: No code may appear between the above clone3 call
    //       and the below async-signal-safe code section.

    /* ================================================================== */
    /*                   BEGIN OF ASYNC-SIGNAL-SAFE CODE                  */
    /* ================================================================== */

    // The code within this section must be async-signal-safe.
    // See the man page signal-safety(7) for details.
    // IMPORTANT: NO HEAP ALLOCATIONS ALLOWED!

    // clone3 returns a pid, but syscall returns a c_long.
    let pid = pid as libc::pid_t;

    // If clone3 returns zero, then we are the child process.
    // NOTE: Once the body of this if statement is entered,
    //       there must be no code paths leading back out!
    if pid == 0 {

        // Unwinding the stack would be horrifying.
        always_abort();

        // Close the read end of the pipe.
        drop(pipe_r);

        // Assert-like function for use within this if statement.
        // If the condition is false, we write an error to the pipe,
        // and then immediately terminate the child process.
        let enforce = |message: &'static str, condition: bool| {
            if !condition {
                let pipe_w = pipe_w.as_raw_fd();
                unsafe {
                    let errnum = (*libc::__errno_location(): i32).to_ne_bytes();
                    libc::write(pipe_w, errnum.as_ptr().cast(), 4);
                    libc::write(pipe_w, message.as_ptr().cast(), message.len());
                    libc::_exit(1);
                }
            }
        };

        // Write the /proc/self/* files prepared above.
        unsafe {
            let write_file = |pathname: &'static [u8], data: &[u8]| {
                let fd = libc::open(pathname.as_ptr().cast(), libc::O_WRONLY, 0);
                enforce("open", fd != -1);
                let nwritten = libc::write(fd, data.as_ptr().cast(), data.len());
                enforce("write", nwritten == data.len() as isize);
                libc::close(fd);
            };
            write_file(b"/proc/self/setgroups\0", setgroups.as_bytes());
            write_file(b"/proc/self/uid_map\0", uid_map.as_bytes());
            write_file(b"/proc/self/gid_map\0", gid_map.as_bytes());
        }

        // Configure the standard streams stdin, stdout, and stderr.
        // dup2 turns off CLOEXEC which is exactly what we need.
        let log = log.as_raw_fd();
        unsafe {
            enforce("close stdin", libc::close(0) != -1);
            enforce("dup2 stdout", libc::dup2(log, 1) != -1);
            enforce("dup2 stderr", libc::dup2(log, 2) != -1);
        }

        // Change the working directory.
        enforce("chdir", unsafe { libc::chdir(scratch_path.as_ptr()) } != -1);

        // Apply the prepared mounts.
        for mount in mounts {
            let mount = unsafe {
                libc::mount(mount.source.as_ptr(), mount.target.as_ptr(),
                            mount.filesystemtype.as_ptr(), mount.mountflags,
                            mount.data.as_ptr().cast())
            };
            enforce("mount", mount != -1);
        }

        // Change the root directory.
        enforce("chroot", unsafe { libc::chroot(b".\0".as_ptr().cast()) } != -1);

        // Change the working directory to the build directory.
        // Must be an absolute path, because chroot doesn't update it.
        let chdir = unsafe { libc::chdir(b"/build\0".as_ptr().cast()) };
        enforce("chdir", chdir != -1);

        // Run the specified program.
        unsafe { libc::execve(program.as_ptr(), execve_argv, execve_envp) };
        enforce("execve", false);
        unreachable!();

    }

    /* ================================================================== */
    /*                    END OF ASYNC-SIGNAL-SAFE CODE                   */
    /* ================================================================== */

    // If clone3 returns -1, something went wrong.
    // In this case, child and pidfd are both not created.
    if pid == -1 {
        return Err(io::Error::last_os_error().into());
    }

    // If any of the code below fails, kill the child.
    // SIGKILL is normally frowned upon; the child gets no chance to clean up.
    // But in our case the child is sandboxed; there is nothing to clean up.
    let child_guard = ScopeExit::new(|| {
        unsafe { libc::kill(pid, libc::SIGKILL); }
        unsafe { libc::waitpid(pid, null_mut(), libc::WNOHANG); }
    });

    // SAFETY: clone3 created a valid file descriptor.
    let pidfd = unsafe { OwnedFd::from_raw_fd(pidfd) };

    // Close the write end of the pipe.
    drop(pipe_w);

    // Read from the read end of the pipe.
    // On EOF, we know that execve was successful.
    // On data, the child has written an error to us.
    let mut buf = [0; 128];
    let nread = File::from(pipe_r).read(&mut buf)?;
    if nread != 0 {
        // First four bytes are errno, remaining bytes are error message.
        let io_error = i32::from_ne_bytes(buf[.. 4].try_into().unwrap());
        let io_error = io::Error::from_raw_os_error(io_error);
        let message = String::from_utf8_lossy(&buf[4 ..]);
        let message = message.trim_end_matches('\0').to_owned();
        return Err(Error::ContainerSetup(io_error, message));
    }

    // A pidfd reports "readable" when the child terminates.
    // We don't need to actually read from the pidfd, only ppoll.
    let mut pollfd = libc::pollfd{
        fd: pidfd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };

    // Convert timeout from Duration to libc::timespec.
    let timeout = libc::timespec{
        tv_sec: timeout.as_secs().try_into().unwrap_or(libc::time_t::MAX),
        tv_nsec: timeout.subsec_nanos().try_into().unwrap_or(libc::c_long::MAX),
    };

    // Wait for the child to terminate or the timeout to occur.
    // We don't handle the case where ppoll fails with EINTR,
    // because we'd need to adjust the timeout on retry,
    // which is a hassle I can't be bothered with.
    let ppoll = unsafe { libc::ppoll(&mut pollfd, 1, &timeout, null()) };
    if ppoll == -1 {
        return Err(io::Error::last_os_error().into());
    }
    if ppoll == 0 {
        return Err(Error::Timeout);
    }

    // The child has terminated, so no need to kill it.
    forget(child_guard);

    // Clean up the child process and obtain its wait status.
    // Check that the child terminated successfully.
    let mut wstatus = 0;
    let waitpid = unsafe { libc::waitpid(pid, &mut wstatus, 0) };
    assert_eq!(waitpid, pid, "pidfd reported that child has terminated");
    ExitStatus::from_raw(wstatus).exit_ok()?;

    Ok(())
}

/// Arguments to the clone3 system call.
///
/// This struct is unfortunately not part of the libc crate.
#[repr(C)]
struct clone_args
{
    flags:        u64,
    pidfd:        u64,
    child_tid:    u64,
    parent_tid:   u64,
    exit_signal:  u64,
    stack:        u64,
    stack_size:   u64,
    tls:          u64,
    set_tid:      u64,
    set_tid_size: u64,
    cgroup:       u64,
}

/// Prepared arguments to mount.
struct PreparedMount
{
    source:         CString,
    target:         CString,
    filesystemtype: CString,
    mountflags:     libc::c_ulong,
    data:           CString,
}

impl PreparedMount
{
    /// Create a mount.
    fn new(into: &mut Vec<Self>, source: &str, target: &str,
           filesystemtype: &str, mountflags: libc::c_ulong, data: &str)
    {
        let this = Self{
            source:         CString::new(source).unwrap(),
            target:         CString::new(target).unwrap(),
            filesystemtype: CString::new(filesystemtype).unwrap(),
            mountflags:     mountflags,
            data:           CString::new(data).unwrap(),
        };
        into.push(this);
    }

    /// Create a read-only bind mount.
    ///
    /// This is more involved than simply passing `MS_BIND | MS_RDONLY`.
    /// See https://unix.stackexchange.com/a/492462 for more information.
    fn new_rdonly_bind(into: &mut Vec<Self>, source: &str, target: &str)
    {
        let flags_1 = libc::MS_BIND | libc::MS_REC;
        let flags_2 = flags_1 | libc::MS_RDONLY | libc::MS_REMOUNT;
        Self::new(into, source, target, "", flags_1, "");
        Self::new(into, "none", target, "", flags_2, "");
    }
}

/// Prepare the argv or envp arguments to `execve`.
///
/// `execve` expects these to be arrays of nul-terminated strings,
/// with a null pointer following the last element of the array.
/// The returned tuple contains a handle and a pointer to the array.
/// The pointer remain valid as long as the handle isn't dropped.
fn prepare_argv_envp<'a, I, A>(cstrings: I)
    -> (*mut *const libc::c_char, impl 'a + Drop)
    where I: IntoIterator<Item=&'a A>
        , A: 'a + AsRef<CStr>
{
    let mut handle: Vec<*const libc::c_char> =
        cstrings
        .into_iter()
        .map(AsRef::as_ref)
        .map(CStr::as_ptr)
        .chain(Some(null()))
        .collect();
    (handle.as_mut_ptr(), handle)
}

#[cfg(test)]
mod tests
{
    use {
        super::*,
        os_ext::{O_DIRECTORY, O_PATH, O_RDWR, O_TMPFILE, mkdtemp, open},
        scope_exit::scope_exit,
        std::{
            assert_matches::assert_matches,
            env::var_os,
            fs::remove_dir_all,
            io::Seek,
            os::unix::io::AsFd,
        },
    };

    /// Call the gist function and return the result and the log file.
    fn call_gist(program: &Path, arguments: &[&str], timeout: Duration)
        -> (Result<(), Error>, File)
    {
        let path = mkdtemp(cstr!(b"/tmp/snowflake-test-XXXXXX")).unwrap();
        scope_exit! { let _ = remove_dir_all(&path); }

        let log = open(cstr!(b"."), O_RDWR | O_TMPFILE, 0o644).unwrap();
        let scratch = open(&path, O_DIRECTORY | O_PATH, 0).unwrap();

        let arguments: Vec<CString> =
            arguments.iter()
            .map(|&s| CString::new(s).unwrap())
            .collect();

        let result = gist(
            log.as_fd(),
            scratch.as_fd(),
            program,
            &arguments,
            &[],
            timeout,
        );

        let mut log = File::from(log);
        log.rewind().unwrap();

        (result, log)
    }

    #[test]
    fn pid_1()
    {
        let bash = var_os("SNOWFLAKE_BASH").unwrap();
        let (result, mut log) = call_gist(
            &Path::new(&bash).join("bin/bash"),
            &["sh", "-c", "echo $$"],
            Duration::from_millis(50),
        );
        assert_matches!(result, Ok(()));
        let mut buf = Vec::new();
        log.read_to_end(&mut buf).unwrap();
        assert_eq!(buf, b"1\n");
    }

    #[test]
    fn timeout()
    {
        let coreutils = var_os("SNOWFLAKE_COREUTILS").unwrap();
        let (result, _) = call_gist(
            &Path::new(&coreutils).join("bin/sleep"),
            &["sleep", "0.060"],
            Duration::from_millis(50),
        );
        assert_matches!(result, Err(Error::Timeout));
    }

    #[test]
    fn unsuccessful_termination()
    {
        let coreutils = var_os("SNOWFLAKE_COREUTILS").unwrap();
        let (result, _) = call_gist(
            &Path::new(&coreutils).join("bin/false"),
            &["false"],
            Duration::from_millis(50),
        );
        assert_matches!(result, Err(Error::ExitStatus(_)));
    }
}
