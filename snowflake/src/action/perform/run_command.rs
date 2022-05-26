use super::{Perform, Result};

#[repr(C)]
enum status
{
    child_terminated,
    failure_clone3,
    failure_ppoll,
    failure_timeout,
    failure_waitpid,
}

extern "C"
{
    fn snowflake_perform_run_command_gist(
        wstatus: &mut libc::c_int,
        timeout: libc::timespec,
    ) -> status;
}

pub fn perform_run_command(perform: &Perform) -> Result
{
    let mut wstatus = 0;
    let timeout = libc::timespec{tv_sec: 1, tv_nsec: 0};
    let status = unsafe {
        snowflake_perform_run_command_gist(&mut wstatus, timeout)
    };
    todo!("{:?} {:?}", status as u32, wstatus)
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
