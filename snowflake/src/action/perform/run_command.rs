use {super::{Perform, Result}, std::{io, os::unix::io::AsRawFd}};

#[repr(C)]
enum status
{
    child_terminated,
    failure_pipe2,
    failure_clone3,
    failure_read,
    failure_pre_execve,
    failure_ppoll,
    failure_timeout,
    failure_waitpid,
}

extern "C"
{
    fn snowflake_perform_run_command_gist(
        wstatus:  &mut libc::c_int,
        errbuf:   *mut u8,
        errlen:   libc::size_t,
        log_file: libc::c_int,
        timeout:  libc::timespec,
    ) -> status;
}

pub fn perform_run_command(perform: &Perform) -> Result
{
    let mut wstatus = 0;
    let mut errbuf = [0; 128];
    let timeout = libc::timespec{tv_sec: 1, tv_nsec: 0};
    let status = unsafe {
        snowflake_perform_run_command_gist(
            &mut wstatus,
            errbuf.as_mut_ptr(),
            errbuf.len(),
            perform.log.as_raw_fd(),
            timeout,
        )
    };
    todo!(
        "{:?} {:?} {:?} {:?}",
        status as u32,
        io::Error::from_raw_os_error(
            i32::from_ne_bytes(errbuf[0 .. 4].try_into().unwrap()),
        ),
        String::from_utf8_lossy(&errbuf[4 ..])
            .trim_end_matches('\0'),
        wstatus,
    )
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
        let outputs = std::fs::File::open("/dev/null").unwrap();
        let perform = Perform{log: log.as_fd(), outputs: outputs.as_fd()};
        perform_run_command(&perform);
    }
}
