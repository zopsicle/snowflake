//! The implementation of `perform_run_command` is largely in C++, because:
//!
//!  - We need to do a lot of obscure syscalls with C strings etc.
//!  - All the code between clone3 and execve must be async-signal-safe.
//!    That section of code must absolutely not perform any heap allocations!
//!
//! All of this is unsafe anyway and all the man pages and examples are in C,
//! so using C++ for this is a little easier than doing it all in unsafe Rust.
//!
//! The implementation does not currently retry on EINTR.
//! This is fine because Snowflake does not use signal handlers.

// _FORTIFY_SOURCE causes warnings when compiling with -O0.
#undef _FORTIFY_SOURCE

#include <cerrno>
#include <cstdint>
#include <cstring>
#include <ctime>
#include <fcntl.h>
#include <linux/sched.h>
#include <optional>
#include <poll.h>
#include <sched.h>
#include <signal.h>
#include <string>
#include <sys/syscall.h>
#include <sys/wait.h>
#include <unistd.h>
#include <utility>

namespace
{
    /// Handy generic scope guard with "skip" and "run now" features.
    template<typename F>
    class scope_exit
    {
    public:
        explicit scope_exit(F f) : f(std::move(f)) { }
        ~scope_exit() { if (f) (*f)(); }
        scope_exit(scope_exit const& other) = delete;
        scope_exit& operator=(scope_exit const& other) = delete;
        void skip()    { f.reset(); }
        void run_now() { this->~scope_exit(); f.reset(); }
    private:
        std::optional<F> f;
    };

    /// Status returned to the caller.
    enum class status
    {
        child_terminated,
        failure_pipe2,
        failure_clone3,
        failure_read,
        failure_pre_execve,
        failure_ppoll,
        failure_timeout,
        failure_waitpid,
    };
}

/// The gist of perform_run_command.
extern "C" status snowflake_perform_run_command_gist(
    int&         wstatus,
    char*        errbuf,
    std::size_t  errlen,
    int          log_file,
    char const*  execve_pathname,
    char* const* execve_argv,
    char* const* execve_envp,
    timespec     timeout
)
{

/* -------------------------------------------------------------------------- */
/*                  Prepare writes to /proc/self/{u,g}id_map                  */
/* -------------------------------------------------------------------------- */

    auto uid_map = "0 " + std::to_string(getuid()) + " 1";
    auto gid_map = "0 " + std::to_string(getgid()) + " 1";

/* -------------------------------------------------------------------------- */
/*                          Create communication pipe                         */
/* -------------------------------------------------------------------------- */

    // This pipe is used by the child to send pre-execve errors to the parent.
    // Once the read-end is closed, the parent knows execve has succeeded.
    int pipefd[2];
    if (pipe2(pipefd, O_CLOEXEC) == -1)
        return status::failure_pipe2;
    scope_exit pipefd0_guard([&] { close(pipefd[0]); });
    scope_exit pipefd1_guard([&] { close(pipefd[1]); });

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

/* ========================================================================== */
/*                       BEGIN OF ASYNC-SIGNAL-SAFE CODE                      */
/* ========================================================================== */

    scope_exit pidfd_guard([&] { close(pidfd); });

/* -------------------------------------------------------------------------- */
/*                         Code that runs in the child                        */
/* -------------------------------------------------------------------------- */

    if (pid == 0) {

        // Close the read end of the pipe.
        pipefd0_guard.run_now();

        // Function for sending error to the parent.
        auto send_error = [&] (char const* source) {
            std::int32_t errnum = errno;
            write(pipefd[1], &errnum, sizeof(errnum));
            write(pipefd[1], source, std::strlen(source));
            _exit(1);
        };

        // Function for dumping data to a file.
        auto write_file = [&] (
            char const* pathname,
            char const* ptr,
            std::size_t len
        ) {
            int fd = open(pathname, O_CLOEXEC | O_WRONLY);
            if (fd == -1) send_error("open");
            if (write(fd, ptr, len) == -1) send_error("write");
            close(fd);
        };

        // Map root inside container to actual user outside container.
        write_file("/proc/self/setgroups", "deny\n", 5);
        write_file("/proc/self/uid_map", uid_map.c_str(), uid_map.size());
        write_file("/proc/self/gid_map", gid_map.c_str(), gid_map.size());

        // Configure standard streams.
        close(0);
        if (dup2(log_file, 1) == -1) send_error("dup2");
        if (dup2(log_file, 2) == -1) send_error("dup2");

        // Start the specified program.
        execve(execve_pathname, execve_argv, execve_envp);
        send_error("execve");

    }

/* ========================================================================== */
/*                        END OF ASYNC-SIGNAL-SAFE CODE                       */
/* ========================================================================== */

    // Clean up the child in case of error.
    // We can call SIGKILL and not worry about unfreed resources,
    // because the child runs in a container (incl. a PID namespace).
    scope_exit child_guard([&] {
        kill(pid, SIGKILL);
        waitpid(pid, nullptr, 0);
    });

/* -------------------------------------------------------------------------- */
/*                             Waiting for execve                             */
/* -------------------------------------------------------------------------- */

    // Close the write end of the pipe.
    pipefd1_guard.run_now();

    int nread = read(pipefd[0], errbuf, errlen);

    if (nread == -1)
        return status::failure_read;

    // If the child sent data over the pipe,
    // something went wrong pre-execve.
    if (nread != 0)
        return status::failure_pre_execve;

    // No longer need the read end of the pipe.
    pipefd0_guard.run_now();

/* -------------------------------------------------------------------------- */
/*                          Implementing the timeout                          */
/* -------------------------------------------------------------------------- */

    pollfd poll_fd{
        .fd = pidfd,
        .events = POLLIN,
        .revents = 0,
    };

    // ppoll will wait until the child terminates, or a timeout occurs.
    int poll_result = ppoll(&poll_fd, 1, &timeout, nullptr);

    if (poll_result == -1)
        return status::failure_ppoll;

    // ppoll returning 0 indicates a timeout.
    if (poll_result == 0)
        return status::failure_timeout;

/* -------------------------------------------------------------------------- */
/*                            Cleaning up the child                           */
/* -------------------------------------------------------------------------- */

    // Even though the child has terminated, we need to call waitpid.
    // This retrieves the wait status and cleans up resources.
    if (waitpid(pid, &wstatus, 0) != pid)
        return status::failure_waitpid;

    // Child has been waited for by now.
    child_guard.skip();

    return status::child_terminated;

}
