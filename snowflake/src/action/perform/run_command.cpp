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

#include <cerrno>
#include <cstdint>
#include <ctime>
#include <linux/sched.h>
#include <poll.h>
#include <sched.h>
#include <signal.h>
#include <sys/syscall.h>
#include <sys/wait.h>
#include <unistd.h>

namespace
{
    enum class status
    {
        child_terminated,
        failure_clone3,
        failure_ppoll,
        failure_timeout,
        failure_waitpid,
    };
}

/// The gist of perform_run_command.
extern "C" status snowflake_perform_run_command_gist(
    int& wstatus,
    timespec timeout
)
{

/* -------------------------------------------------------------------------- */
/*                               Invoking clone3                              */
/* -------------------------------------------------------------------------- */

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

    // Otherwise waitpid fails with ECHILD.
    cl_args.exit_signal = SIGCHLD;

    // Interface of this syscall is similar to that of fork(2).
    pid_t pid = syscall(SYS_clone3, &cl_args, sizeof(cl_args));

    if (pid == -1)
        return status::failure_clone3;

/* -------------------------------------------------------------------------- */
/*                         Code that runs in the child                        */
/* -------------------------------------------------------------------------- */

    if (pid == 0) {
        _exit(1);
    }

/* -------------------------------------------------------------------------- */
/*                          Implementing the timeout                          */
/* -------------------------------------------------------------------------- */

    auto kill_child = [&] {
        kill(pid, SIGKILL);
        waitpid(pid, nullptr, 0);
    };

    pollfd poll_fd{
        .fd = pidfd,
        .events = POLLIN,
        .revents = 0,
    };

    int poll_result = ppoll(&poll_fd, 1, &timeout, nullptr);

    // Snowflake does not use signal handlers.
    // So there is no need to handle EINTR.
    if (poll_result == -1) {
        kill_child();
        return status::failure_ppoll;
    }

    // ppoll returning 0 indicates a timeout.
    if (poll_result == 0) {
        kill_child();
        return status::failure_timeout;
    }

    // ppoll returning non-0 indicates the child terminated.
    int waitpid_pid = waitpid(pid, &wstatus, 0);
    if (waitpid_pid != pid) {
        kill_child();
        return status::failure_waitpid;
    }

    return status::child_terminated;

}
