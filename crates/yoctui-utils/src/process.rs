use std::io;

/// Return whether a failed spawn can succeed once the executable stops being
/// rewritten by another process.
#[cfg(unix)]
pub fn is_transient_spawn_error(error: &io::Error) -> bool {
    error.raw_os_error() == Some(libc::ETXTBSY)
}

#[cfg(not(unix))]
pub fn is_transient_spawn_error(_error: &io::Error) -> bool {
    false
}

/// Best-effort Unix process-priority reduction for background discovery work.
/// A larger nice value gives interactive work precedence. Callers choose
/// whether inability to lower priority should fail the operation.
#[cfg(unix)]
pub fn lower_process_priority(pid: u32, nice: i32) -> io::Result<()> {
    if !(0..=19).contains(&nice) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "nice value must be between 0 and 19",
        ));
    }
    // SAFETY: setpriority receives a process id returned by spawn and a
    // validated value. It neither borrows memory nor retains pointers.
    if unsafe { libc::setpriority(libc::PRIO_PROCESS, pid, nice) } == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(not(unix))]
pub fn lower_process_priority(_pid: u32, nice: i32) -> io::Result<()> {
    if !(0..=19).contains(&nice) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "nice value must be between 0 and 19",
        ));
    }
    Ok(())
}

/// Kill an owned child process group when an asynchronous operation is dropped.
/// Callers must start the child in a new group whose ID equals its PID, and
/// disarm after normal completion. The child handle remains responsible for reaping.
pub struct ProcessGroupGuard {
    pid: Option<i32>,
}

impl ProcessGroupGuard {
    pub fn new(pid: u32) -> Self {
        Self {
            pid: i32::try_from(pid).ok().filter(|pid| *pid > 0),
        }
    }

    pub fn disarm(&mut self) {
        self.pid = None;
    }
}

impl Drop for ProcessGroupGuard {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(pid) = self.pid {
            // SAFETY: callers supply the PID of their own newly isolated group.
            unsafe {
                libc::kill(-pid, libc::SIGKILL);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn process_group_guard_terminates_owned_child_on_drop() {
        use std::os::unix::process::{CommandExt, ExitStatusExt};
        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .process_group(0)
            .spawn()
            .unwrap();
        drop(ProcessGroupGuard::new(child.id()));
        assert_eq!(child.wait().unwrap().signal(), Some(libc::SIGKILL));
        drop(ProcessGroupGuard::new(0));
        drop(ProcessGroupGuard::new(u32::MAX));
    }

    #[test]
    fn priority_rejects_values_outside_the_portable_nice_range() {
        assert_eq!(
            lower_process_priority(u32::MAX, -1).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            lower_process_priority(u32::MAX, 20).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }

    #[cfg(unix)]
    #[test]
    fn spawned_background_process_receives_requested_priority() {
        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .spawn()
            .unwrap();
        lower_process_priority(child.id(), 10).unwrap();
        // SAFETY: getpriority reads kernel state for the live child PID.
        let nice = unsafe { libc::getpriority(libc::PRIO_PROCESS, child.id()) };
        let _ = child.kill();
        let _ = child.wait();
        assert_eq!(nice, 10);
    }
}
