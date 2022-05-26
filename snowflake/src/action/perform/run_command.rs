use {
    crate::basename::Basename,
    super::{Error, Perform, Result, Summary},
    os_ext::{getgid, getuid, pipe2},
    scope_exit::ScopeExit,
    std::{
        borrow::Cow,
        ffi::{CStr, CString},
        fs::File,
        io::{self, Read},
        mem::{forget, size_of_val, zeroed},
        os::unix::{
            ffi::OsStrExt,
            io::{AsRawFd, FromRawFd, OwnedFd},
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

pub fn perform_run_command<'a>(
    perform: &Perform,
    outputs: &'a [Arc<Basename>],
    program: &Path,
    arguments: &[CString],
    environment: &[CString],
    timeout: Duration,
) -> Result<'a>
{
    // Prepare writes to /proc/self/gid_map and /proc/self/uid_map.
    // These files map users and groups inside the container
    // to users and groups outside the container.
    let setgroups = "deny\n";
    let uid_map = format!("0 {} 1\n", getuid());
    let gid_map = format!("0 {} 1\n", getgid());

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
                unsafe {
                    let pipe_w = pipe_w.as_raw_fd();
                    let errnum = (*libc::__errno_location(): i32).to_ne_bytes();
                    libc::write(pipe_w, errnum.as_ptr().cast(), 4);
                    libc::write(pipe_w, message.as_ptr().cast(), message.len());
                    libc::_exit(1);
                }
            }
        };

        // Configure standard streams.
        let log = perform.log.as_raw_fd();
        unsafe {
            enforce("close stdin", libc::close(0) != -1);
            enforce("dup2 stdout", libc::dup2(log, 1) != -1);
            enforce("dup2 stderr", libc::dup2(log, 2) != -1);
        }

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

    // Collect all the output paths.
    let outputs_dir =
        Path::new("outputs");
    let outputs =
        outputs.iter()
        .map(|b| outputs_dir.join(&**b))
        .map(|p| Cow::Owned(p))
        .collect();

    Ok(Summary{outputs, warnings: false})
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
    use super::*;

    #[test]
    fn f()
    {
        use std::os::unix::io::AsFd;
        let log = std::fs::File::open("/dev/null").unwrap();
        let scratch = std::fs::File::open("/dev/null").unwrap();
        let perform = Perform{log: log.as_fd(), scratch: scratch.as_fd()};
        perform_run_command(
            &perform,
            &[],
            Path::new("/run/current-system/sw/bin/sleep"),
            &[
                CString::new("sleep").unwrap(),
                CString::new("2").unwrap(),
            ],
            &[],
            Duration::from_secs(1),
        ).unwrap();
    }
}
