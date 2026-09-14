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

#[cfg(test)]
mod tests {
    use super::*;

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
