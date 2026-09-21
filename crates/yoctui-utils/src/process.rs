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
#[path = "tests/process/mod.rs"]
mod tests;
