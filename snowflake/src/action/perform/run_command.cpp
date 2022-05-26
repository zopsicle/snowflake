//! The implementation of `perform_run_command` is largely in C++, because:
//!
//!  - We need to do a lot of obscure syscalls with C strings etc.
//!  - All the code between clone3 and execve must be async-signal-safe.
//!    That section of code must absolutely not perform any heap allocations!
//!
//! All of this is unsafe anyway and all the man pages and examples are in C,
//! so using C++ for this is a little easier than doing it all in unsafe Rust.

// _FORTIFY_SOURCE causes warnings when compiling with -O0.
#undef _FORTIFY_SOURCE

#include <cstdint>
#include <linux/sched.h>
#include <sched.h>
#include <sys/syscall.h>
#include <unistd.h>

/// The gist of perform_run_command.
extern "C" void snowflake_perform_run_command_gist()
{
    // Zero-initialize this because we don't use most of its features.
    clone_args cl_args{};

    // Enable all the namespace features.
    cl_args.flags |= (
        CLONE_NEWCGROUP |  // New cgroup namespace.
        CLONE_NEWIPC    |  // New IPC namespace.
        CLONE_NEWNET    |  // New network namespace.
        CLONE_NEWNS     |  // New mount namespace.
        CLONE_NEWPID    |  // New PID namespace.
        CLONE_NEWUSER   |  // New user namespace.
        CLONE_NEWUTS       // New UTS namespace.
    );

    // Create pidfd for use with poll(2).
    int pidfd;
    cl_args.flags |= CLONE_PIDFD;
    cl_args.pidfd = reinterpret_cast<std::uint64_t>(&pidfd);

    // Interface of this syscall is similar to that of fork(2).
    pid_t pid = syscall(SYS_clone3, &cl_args, sizeof(cl_args));

    if (pid == -1) {
        // TODO: Return error.
    }

    if (pid == 0) {
        // In child.
        _exit(1);
    }

    // In parent.
}
