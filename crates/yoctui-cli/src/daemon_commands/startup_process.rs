//! Owned startup child cleanup, private diagnostics and terminal-independent launch.
use super::*;

pub(crate) struct DaemonStartupChild {
    pub(crate) child: std::process::Child,
    pub(crate) ready: bool,
}

impl Drop for DaemonStartupChild {
    fn drop(&mut self) {
        if self.ready || self.child.try_wait().ok().flatten().is_some() {
            return;
        }
        // Only the unreaped foreground child spawned by this startup attempt.
        unsafe {
            libc::kill(self.child.id() as i32, libc::SIGTERM);
        }
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if self.child.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub(super) fn open_daemon_log(path: &Path) -> Result<fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    let log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(path)
        .with_context(|| format!("could not open daemon diagnostics {}", path.display()))?;
    anyhow::ensure!(
        log.metadata()?.is_file(),
        "daemon diagnostics must be a regular file"
    );
    log.set_permissions(fs::Permissions::from_mode(0o600))?;
    Ok(log)
}

pub(super) fn detach_daemon_session(command: &mut ProcessCommand) {
    // setsid is async-signal-safe; the child owns neither the parent's session
    // nor its controlling terminal. Foreground mode intentionally remains attached.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                Err(io::Error::last_os_error())
            } else {
                Ok(())
            }
        });
    }
}
