use {
    anyhow::Context,
    os_ext::{
        AT_SYMLINK_NOFOLLOW,
        S_IFDIR, S_IFLNK, S_IFMT, S_IFREG,
        cstr, cstr::IntoCStr, cstr_cow,
        io::BorrowedFdExt,
        fstatat, getgid, getuid, mkdirat, mknodat,
        pipe2, readlink, readlinkat, symlinkat,
    },
    regex::bytes::Regex,
    scope_exit::ScopeExit,
    snowflake_core::action::{
        Action, Error, Outputs, Perform, Success,
        Result as AResult,
    },
    snowflake_util::{basename::Basename, hash::{Blake3, Hash}},
    std::{
        borrow::Cow,
        ffi::{CStr, CString, NulError, OsString},
        fs::File,
        io::{self, BufRead, BufReader, Read, Seek},
        mem::{forget, size_of_val, zeroed},
        ops::Deref,
        os::unix::{
            ffi::OsStringExt,
            io::{AsRawFd, BorrowedFd, FromRawFd, OwnedFd},
            process::ExitStatusExt,
        },
        panic::always_abort,
        path::{Path, PathBuf},
        process::ExitStatus,
        ptr::{addr_of, addr_of_mut, null, null_mut},
        time::Duration,
    },
};

/// Action that runs an arbitrary command in a container.
pub struct RunCommand
{
    /// What to call the inputs in the command's working directory.
    pub inputs: Vec<Basename<OsString>>,

    /// What the outputs are called in the command's working directory.
    pub outputs: Outputs<Vec<Basename<OsString>>>,

    /// Absolute path to the program to run.
    pub program: PathBuf,

    /// Arguments to the program.
    ///
    /// This should include the zeroth argument,
    /// which is normally equal to [`program`][`Self::program`].
    pub arguments: Vec<CString>,

    /// The environment variables to the program.
    ///
    /// This specifies the *exact* environment to the program.
    /// No extra environment variables are set by the
    /// [`perform`][`RunCommand::perform`] method.
    pub environment: Vec<CString>,

    /// How much time the program may spend.
    ///
    /// If the program spends more time than this,
    /// it is killed and the action fails.
    pub timeout: Duration,

    /// Regular expression that matches warnings in the build log.
    ///
    /// If [`None`], no warnings are assumed to have been emitted.
    pub warnings: Option<Regex>,
}

impl Action for RunCommand
{
    fn inputs(&self) -> usize
    {
        self.inputs.len()
    }

    fn outputs(&self) -> Outputs<usize>
    {
        self.outputs.as_ref().map(Vec::len)
    }

    fn perform(&self, perform: &Perform, input_paths: &[PathBuf]) -> AResult
    {
        perform_run_command(perform, self, input_paths)
    }

    fn hash(&self, input_hashes: &[Hash]) -> Hash
    {
        // NOTE: See the manual chapter on avoiding hash collisions.

        const OUTPUTS_TYPE_OUTPUTS: u8 = 0;
        const OUTPUTS_TYPE_LINT:    u8 = 1;

        let Self{inputs, outputs, program, arguments,
                 environment, timeout, warnings} = self;

        debug_assert_eq!(input_hashes.len(), inputs.len());

        let mut h = Blake3::new();

        h.put_str("RunCommand");

        h.put_usize(inputs.len());
        for (basename, hash) in inputs.iter().zip(input_hashes) {
            h.put_os_str(basename);
            h.put_hash(*hash);
        }

        match outputs {
            Outputs::Outputs(outputs) => {
                h.put_u8(OUTPUTS_TYPE_OUTPUTS);
                h.put_slice(outputs, |h, o| h.put_os_str(o));
            },
            Outputs::Lint => {
                h.put_u8(OUTPUTS_TYPE_LINT);
            },
        }

        h.put_path(program);
        h.put_slice(arguments, |h, a| h.put_cstr(a));
        h.put_slice(environment, |h, e| h.put_cstr(e));

        // The timeout cannot affect the output of the action,
        // so there is no need to include it in the hash.
        let _ = timeout;

        h.put_bool(warnings.is_some());
        if let Some(warnings) = warnings {
            h.put_str(warnings.as_str());
        }

        h.finalize()
    }
}

fn perform_run_command(
    perform: &Perform,
    action: &RunCommand,
    input_paths: &[PathBuf],
) -> AResult
{
    // Unpack the arguments into convenient variables.
    let Perform{build_log, source_root, scratch} = perform;
    let RunCommand{inputs, outputs, program, arguments,
                   environment, timeout, warnings} = action;

    // Mounting must happen in the child process,
    // so we collect all the mount calls in here.
    // Targets are relative to scratch directory.
    let mut mounts = Vec::new();

    // Perform the run command action.
    let scratch_path = resolve_magic(*scratch)                                  .with_context(|| "Find path to scratch directory")?;
    populate_root_directory(*scratch)?;
    populate_dev_directory(*scratch, &mut mounts)?;
    install_blessed_programs(*scratch)?;
    repair_root_mount(&mut mounts);
    mount_proc(&mut mounts);
    mount_nix_store(&mut mounts);
    mount_inputs(*source_root, *scratch, inputs, input_paths, &mut mounts)?;
    run_command(*build_log, &scratch_path, program,
                arguments, environment, *timeout,
                mounts)?;
    let output_paths = output_paths(outputs);
    let warnings = find_warnings(*build_log, warnings.as_ref())?;

    // Summarize the result.
    Ok(Success{output_paths, warnings})
}

/// Arguments to mount.
#[derive(Default)]
struct Mount<'a>
{
    source: Cow<'a, CStr>,
    target: Cow<'a, CStr>,
    filesystemtype: Cow<'a, CStr>,
    mountflags: libc::c_ulong,
    data: Cow<'a, CStr>,
}

impl<'a> Mount<'a>
{
    /// Create a read-only bind mount.
    ///
    /// This is more involved than simply passing `MS_BIND | MS_RDONLY`.
    /// See https://unix.stackexchange.com/a/492462 for more information.
    pub fn rdonly_bind_mount(
        source: impl IntoCStr<'a>,
        target: impl Clone + IntoCStr<'a>,
    ) -> Result<[Self; 2], NulError>
    {
        let common_flags = libc::MS_BIND | libc::MS_REC;
        Ok([
            Self{
                source: source.into_cstr()?,
                target: target.clone().into_cstr()?,
                mountflags: common_flags,
                ..Mount::default()
            },
            Self{
                source: cstr_cow!(b"none"),
                target: target.into_cstr()?,
                mountflags: common_flags | libc::MS_RDONLY | libc::MS_REMOUNT,
                ..Mount::default()
            },
        ])
    }
}

/// Populate the container's `/` directory.
fn populate_root_directory(scratch: BorrowedFd) -> Result<(), Error>
{
    let mk = |path, mode| mkdirat(Some(scratch), path, mode)                    .with_context(|| format!("Create {path:?} inside container"));
    mk(cstr!(b"bin"),       0o755)?;
    mk(cstr!(b"dev"),       0o755)?;
    mk(cstr!(b"nix"),       0o755)?;
    mk(cstr!(b"nix/store"), 0o755)?;
    mk(cstr!(b"proc"),      0o555)?;  // (sic)
    mk(cstr!(b"root"),      0o755)?;
    mk(cstr!(b"usr"),       0o755)?;
    mk(cstr!(b"usr/bin"),   0o755)?;
    mk(cstr!(b"build"),     0o755)?;
    Ok(())
}

/// Populate the container's `/dev` directory.
fn populate_dev_directory(scratch: BorrowedFd, mounts: &mut Vec<Mount>)
    -> Result<(), Error>
{
    // The standard streams are symbolic links, so they cannot be mounted.
    let mk = |target, path| symlinkat(target, Some(scratch), path)              .with_context(|| format!("Create {path:?} inside container"));
    mk(cstr!(b"/proc/self/fd/0"), cstr!(b"dev/stdin"))?;
    mk(cstr!(b"/proc/self/fd/1"), cstr!(b"dev/stdout"))?;
    mk(cstr!(b"/proc/self/fd/2"), cstr!(b"dev/stderr"))?;

    // Device files cannot be created directly; mknod fails with EPERM.
    // Instead, we mknod a regular file for each device file,
    // which we then use as a mount target for a bind mount.
    let device_files = ["null", "zero", "full", "random", "urandom"];
    for basename in device_files {
        let source = Path::new("/dev").join(basename);
        let target = Path::new("dev").join(basename);

        mknodat(Some(scratch), &target, S_IFREG | 0o644, 0)                     .with_context(|| format!("Create {target:?} inside container"))?;

        let mount = Mount{
            source: source.into_cstr().unwrap(),
            target: target.into_cstr().unwrap(),
            mountflags: libc::MS_BIND | libc::MS_REC,
            ..Mount::default()
        };
        mounts.push(mount);
    }

    Ok(())
}

/// Point the container's symbolic links `/bin/sh` and `/usr/bin/env`
/// to their respective executables in the Nix store.
///
/// These two programs are commonly used in shebangs,
/// so having them not be where expected is very annoying.
fn install_blessed_programs(scratch: BorrowedFd) -> Result<(), Error>
{
    static SH:  &str = concat!(env!("SNOWFLAKE_BASH"),      "/bin/bash");
    static ENV: &str = concat!(env!("SNOWFLAKE_COREUTILS"), "/bin/env");

    let mk = |target: &CStr, path| symlinkat(target, Some(scratch), path)       .with_context(|| format!("Create {path:?} inside container"));
    mk(&SH.into_cstr().unwrap(),  cstr!(b"bin/sh"))?;
    mk(&ENV.into_cstr().unwrap(), cstr!(b"usr/bin/env"))?;

    Ok(())
}

/// Prevent mount events from propagating out of the container.
///
/// `/` is usually mounted with `MS_SHARED`, but we want `MS_PRIVATE`.
/// Otherwise, mount events inside the container could propagate out.
fn repair_root_mount(mounts: &mut Vec<Mount>)
{
    let mount = Mount{
        source: cstr_cow!(b"none"),
        target: cstr_cow!(b"/"),
        mountflags: libc::MS_PRIVATE | libc::MS_REC,
        ..Mount::default()
    };
    mounts.push(mount);
}

// Mount the proc file system at the container's path `/proc`.
fn mount_proc(mounts: &mut Vec<Mount>)
{
    let mount = Mount{
        source: cstr_cow!(b"proc"),
        target: cstr_cow!(b"proc"),
        filesystemtype: cstr_cow!(b"proc"),
        mountflags: libc::MS_NODEV | libc::MS_NOEXEC | libc::MS_NOSUID,
        ..Mount::default()
    };
    mounts.push(mount);
}

/// Mount the Nix store at the container's path `/nix/store`.
fn mount_nix_store(mounts: &mut Vec<Mount>)
{
    let mountz = Mount::rdonly_bind_mount("/nix/store", "nix/store").unwrap();
    mounts.extend(mountz);
}

/// Mount every input in the container's `/build` directory.
fn mount_inputs(
    source_root: BorrowedFd,
    scratch: BorrowedFd,
    inputs: &[Basename<OsString>],
    input_paths: &[PathBuf],
    mounts: &mut Vec<Mount>,
) -> Result<(), Error>
{
    debug_assert_eq!(input_paths.len(), inputs.len());

    let source_root_path =
        resolve_magic(source_root)
        .map(CString::into_bytes)
        .map(OsString::from_vec)
        .map(PathBuf::from)
        .with_context(|| "Find path to source root")?;

    for (input_basename, input_path) in inputs.iter().zip(input_paths) {
        mount_input(&source_root_path, scratch, input_basename,
                    input_path, mounts)
            .with_context(|| format!("Mount input {input_path:?} \
                                      at {input_basename:?}"))?;
    }

    Ok(())
}

/// Mount an input in the container's `/build` directory.
fn mount_input(
    source_root_path: &Path,
    scratch: BorrowedFd,
    input_basename: &Basename<OsString>,
    input_path: &Path,
    mounts: &mut Vec<Mount>,
) -> anyhow::Result<()>
{
    // Make the input path absolute so it can be mounted.
    let input_path = source_root_path.join(input_path);

    // Make the target relative to the /build directory.
    let target = Path::new("build").join(input_basename.deref());

    // How to mount the input depends on what type of file it is.
    let statbuf = fstatat(None, &input_path, AT_SYMLINK_NOFOLLOW)               .with_context(|| "Find file type of input")?;
    match statbuf.st_mode & S_IFMT {
        S_IFREG => {
            // If it's a regular file, the target must be a regular file.
            mknodat(Some(scratch), &target, S_IFREG | 0o644, 0)                 .with_context(|| "Create mount target")?;
            let mount = Mount::rdonly_bind_mount(input_path, target)            .with_context(|| "Prepare mount arguments")?;
            mounts.extend(mount);
        },
        S_IFDIR => {
            // If it's a directory, the target must be a directory.
            mkdirat(Some(scratch), &target, 0o755)                              .with_context(|| "Create mount target")?;
            let mount = Mount::rdonly_bind_mount(input_path, target)            .with_context(|| "Prepare mount arguments")?;
            mounts.extend(mount);
        },
        S_IFLNK => {
            // If it's a symbolic link, we're fucked as they can't be mounted.
            // Copy the symbolic link instead (should be fast; they're small).
            let symlink_target = readlinkat(None, input_path)                   .with_context(|| "Find target of symbolic link")?;
            symlinkat(&symlink_target, Some(scratch), target)                   .with_context(|| "Create copy of symbolic link")?;
        },
        _ =>
            unreachable!("Input has been hashed by the driver,
                          so it can only be of a supported type"),
    }

    Ok(())
}

/// Compute the scratch-relative path at which each output is created.
fn output_paths(outputs: &Outputs<Vec<Basename<OsString>>>) -> Vec<PathBuf>
{
    match outputs {
        Outputs::Outputs(outputs) => {
            let build_dir = Path::new("build");
            outputs.iter()
                .map(|b| build_dir.join(&**b))
                .collect()
        },
        Outputs::Lint =>
            Vec::new(),
    }
}

/// Look for warnings in the build log.
fn find_warnings(build_log: BorrowedFd, warnings: Option<&Regex>)
    -> Result<bool, Error>
{
    // If no warnings pattern was given,
    // we assume there were no warnings.
    let Some(warnings) = warnings
        else { return Ok(false) };

    // Open the build log file and rewind it.
    let build_log = build_log.try_to_owned()                                    .with_context(|| "Duplicate build log file descriptor")?;
    let mut build_log = File::from(build_log);
    build_log.rewind()                                                          .with_context(|| "Rewind build log")?;

    // Read lines from the build log file
    // and match them against the pattern.
    let mut build_log = BufReader::new(build_log);
    let mut line = Vec::new();
    loop {
        // BufRead::lines is inadequate as build logs may be invalid UTF-8.
        // That's also why we use regex::bytes::Regex instead of regex::Regex.
        build_log.read_until(b'\n', &mut line)                                  .with_context(|| "Read line from build log")?;
        if line.is_empty()          { break Ok(false); }
        if line.ends_with(b"\n")    { line.pop();      }
        if warnings.is_match(&line) { break Ok(true);  }
        line.clear();
    }
}

/// Obtain the path to the file referred to by a file descriptor.
///
/// Some system calls do not work well with file descriptors:
///
///  1. There is no `mountat` system call, necessitating the use of `mount`.
///  2. `fchdir` doesn't work with mount namespaces for whichever reason.
fn resolve_magic(fd: BorrowedFd) -> Result<CString, Error>
{
    let magic = format!("/proc/self/fd/{}", fd.as_raw_fd());
    readlink(magic).map_err(|err| Error::from(anyhow::Error::from(err)))
}

/// Run the command in the already set up container.
fn run_command(
    build_log: BorrowedFd,
    scratch_path: &CStr,
    program: &Path,
    arguments: &[CString],
    environment: &[CString],
    timeout: Duration,
    // By value, to prevent accidentally adding
    // mounts *after* running the command. :)
    mounts: Vec<Mount>,
) -> Result<(), Error>
{
    // Prepare writes to /proc/self/gid_map and /proc/self/uid_map.
    // These files map users and groups inside the container
    // to users and groups outside the container.
    let setgroups = "deny\n";
    let uid_map = format!("0 {} 1\n", getuid());
    let gid_map = format!("0 {} 1\n", getgid());

    // Prepare arguments to execve.
    let program = program.into_cstr()                                           .with_context(|| "Prepare path to program")?;
    let (execve_argv, _execve_argv) = prepare_argv_envp(arguments);
    let (execve_envp, _execve_envp) = prepare_argv_envp(environment);

    // This pipe is used by the child to send pre-execve errors to the parent.
    // Since CLOEXEC is enabled, the parent knows execve has succeeded.
    let (pipe_r, pipe_w) = pipe2(0)                                             .with_context(|| "Create pipe for parent-child communication")?;

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

        // Write the /proc/self/\* files prepared above.
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
        let build_log = build_log.as_raw_fd();
        unsafe {
            enforce("close stdin", libc::close(0) != -1);
            enforce("dup2 stdout", libc::dup2(build_log, 1) != -1);
            enforce("dup2 stderr", libc::dup2(build_log, 2) != -1);
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
        let error = io::Error::last_os_error();
        return Err(anyhow::Error::from(error))
            .with_context(|| "Fork child process")
            .map_err(Error::from);
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
    let nread = File::from(pipe_r).read(&mut buf)                               .with_context(|| "Read from pipe")?;
    if nread != 0 {
        // First four bytes are errno, remaining bytes are error message.
        let io_error = i32::from_ne_bytes(buf[.. 4].try_into().unwrap());
        let io_error = io::Error::from_raw_os_error(io_error);
        let message = String::from_utf8_lossy(&buf[4 ..]);
        let message = message.trim_end_matches('\0').to_owned();
        return Err(anyhow::Error::from(io_error))
            .with_context(|| message)
            .with_context(|| "Post-fork pre-execve setup")
            .map_err(Error::from);
    }

    // A pidfd reports "readable" when the child terminates.
    // We don't need to actually read from the pidfd, only ppoll.
    let mut pollfd = libc::pollfd{
        fd: pidfd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };

    // Convert timeout from Duration to libc::timespec.
    let ptimeout = libc::timespec{
        tv_sec: timeout.as_secs().try_into().unwrap_or(libc::time_t::MAX),
        tv_nsec: timeout.subsec_nanos().try_into().unwrap_or(libc::c_long::MAX),
    };

    // Wait for the child to terminate or the timeout to occur.
    let ppoll = unsafe { libc::ppoll(&mut pollfd, 1, &ptimeout, null()) };
    if ppoll == -1 {
        let error = io::Error::last_os_error();
        return Err(anyhow::Error::from(error))
            .with_context(|| "Poll child process")
            .map_err(Error::from);
    }
    if ppoll == 0 {
        return Err(Error::Timeout(timeout));
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
        std::{assert_matches::assert_matches, io::Seek, ops::Deref, os::unix::io::AsFd},
    };

    /// Call the perform_run_command function and
    /// return the result and the build log file.
    fn call_perform_run_command(
        source_root: BorrowedFd,
        action: &RunCommand,
        input_paths: &[PathBuf],
    ) -> (Result<Success, Error>, File)
    {
        let path      = mkdtemp(cstr!(b"/tmp/snowflake-test-XXXXXX")).unwrap();
        let build_log = open(cstr!(b"."), O_RDWR | O_TMPFILE, 0o644).unwrap();
        let scratch   = open(&path, O_DIRECTORY | O_PATH, 0).unwrap();

        let perform = Perform{
            build_log: build_log.as_fd(),
            source_root,
            scratch: scratch.as_fd(),
        };

        let result = perform_run_command(&perform, action, input_paths);

        let mut build_log = File::from(build_log);
        build_log.rewind().unwrap();

        (result, build_log)
    }

    #[test]
    fn inputs()
    {
        let coreutils = env!("SNOWFLAKE_COREUTILS");

        let inputs: Vec<Basename<OsString>> =
            ["regular.txt", "directory", "symlink.lnk", "broken.lnk"]
            .into_iter()
            .map(|i| Basename::new(i.into()).unwrap())
            .collect();

        let source_root =
            open("testdata/inputs", O_DIRECTORY | O_PATH, 0)
                .unwrap();

        let input_paths: Vec<PathBuf> =
            inputs.iter()
            .map(|i| i.deref().into())
            .collect();

        let action = RunCommand{
            inputs,
            outputs: Outputs::Outputs(vec![]),
            program: "/bin/sh".into(),
            arguments: vec![
                cstr!(b"sh").into(),
                cstr!(b"-c").into(),
                cstr!(br#"
                    cat regular.txt
                    ls directory
                    readlink symlink.lnk
                    readlink broken.lnk
                "#).into(),
            ],
            environment: vec![
                format!("PATH={coreutils}/bin").into_cstr().unwrap().into(),
            ],
            timeout: Duration::from_millis(50),
            warnings: None,
        };

        let (result, mut build_log) =
            call_perform_run_command(
                source_root.as_fd(),
                &action,
                &input_paths,
            );

        assert_matches!(result, Ok(Success{warnings: false, ..}));
        let mut buf = String::new();
        build_log.read_to_string(&mut buf).unwrap();
        assert_eq!(buf, "Hello, world!\nbar.txt\nfoo.txt\n\
                         regular.txt\nenoent.txt\n");
    }

    #[test]
    fn pid_1()
    {
        let action = RunCommand{
            inputs: vec![],
            outputs: Outputs::Outputs(vec![]),
            program: "/bin/sh".into(),
            arguments: vec![
                cstr!(b"sh").into(),
                cstr!(b"-c").into(),
                cstr!(b"echo $$").into(),
            ],
            environment: vec![],
            timeout: Duration::from_millis(50),
            warnings: None,
        };
        let source_root = open("/dev/null", O_PATH, 0).unwrap();
        let (result, mut build_log) =
            call_perform_run_command(source_root.as_fd(), &action, &[]);
        assert_matches!(result, Ok(Success{warnings: false, ..}));
        let mut buf = Vec::new();
        build_log.read_to_end(&mut buf).unwrap();
        assert_eq!(buf, b"1\n");
    }

    #[test]
    fn timeout()
    {
        let coreutils = env!("SNOWFLAKE_COREUTILS");
        let action = RunCommand{
            inputs: vec![],
            outputs: Outputs::Outputs(vec![]),
            program: Path::new(&coreutils).join("bin/sleep"),
            arguments: vec![cstr!(b"sleep").into(), cstr!(b"0.060").into()],
            environment: vec![],
            timeout: Duration::from_millis(50),
            warnings: None,
        };
        let source_root = open("/dev/null", O_PATH, 0).unwrap();
        let (result, _) =
            call_perform_run_command(source_root.as_fd(), &action, &[]);
        assert_matches!(result, Err(Error::Timeout(_)));
    }

    #[test]
    fn unsuccessful_termination()
    {
        let coreutils = env!("SNOWFLAKE_COREUTILS");
        let action = RunCommand{
            inputs: vec![],
            outputs: Outputs::Outputs(vec![]),
            program: Path::new(&coreutils).join("bin/false"),
            arguments: vec![cstr!(b"false").into()],
            environment: vec![],
            timeout: Duration::from_millis(50),
            warnings: None,
        };
        let source_root = open("/dev/null", O_PATH, 0).unwrap();
        let (result, _) =
            call_perform_run_command(source_root.as_fd(), &action, &[]);
        assert_matches!(result, Err(Error::ExitStatus(_)));
    }

    #[test]
    fn warnings()
    {
        let action = RunCommand{
            inputs: vec![],
            outputs: Outputs::Outputs(vec![]),
            program: "/bin/sh".into(),
            arguments: vec![
                cstr!(b"sh").into(),
                cstr!(b"-c").into(),
                cstr!(b"echo hello; echo 'warning: boo'").into(),
            ],
            environment: vec![],
            timeout: Duration::from_millis(50),
            warnings: Some(Regex::new("^warning:").unwrap()),
        };
        let source_root = open("/dev/null", O_PATH, 0).unwrap();
        let (result, _) =
            call_perform_run_command(source_root.as_fd(), &action, &[]);
        assert_matches!(result, Ok(Success{warnings: true, ..}));
    }
}
