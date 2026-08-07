use std::{
    io,
    os::fd::FromRawFd,
    os::unix::process::CommandExt,
    path::Path,
    process::{Child, Command, Stdio},
};

pub struct PtyShell {
    master: std::fs::File,
    child: Child,
}

impl PtyShell {
    pub fn spawn(shell: &Path, cwd: &Path) -> io::Result<Self> {
        let mut master = -1;
        let mut slave = -1;
        if unsafe {
            libc::openpty(
                &mut master,
                &mut slave,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null(),
            )
        } != 0
        {
            return Err(io::Error::last_os_error());
        }
        let child_fd = slave;
        let child = unsafe {
            Command::new(shell)
                .current_dir(cwd)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .pre_exec(move || {
                    for fd in [0, 1, 2] {
                        libc::dup2(child_fd, fd);
                    }
                    libc::close(child_fd);
                    Ok(())
                })
                .spawn()?
        };
        unsafe {
            libc::close(slave);
        }
        Ok(Self {
            master: unsafe { std::fs::File::from_raw_fd(master) },
            child,
        })
    }
    pub fn resize(&self, width: u16, height: u16) -> io::Result<()> {
        let size = libc::winsize {
            ws_row: height,
            ws_col: width,
            ws_xpixel: 0,
            ws_ypixel: 0,
        };
        let fd = std::os::fd::AsRawFd::as_raw_fd(&self.master);
        if unsafe { libc::ioctl(fd, libc::TIOCSWINSZ, &size) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    }
    pub fn child_id(&self) -> u32 {
        self.child.id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pty_backend_spawns_real_shell_and_propagates_resize() {
        let mut shell = PtyShell::spawn(Path::new("/bin/sh"), Path::new("/tmp")).unwrap();
        shell.resize(80, 24).unwrap();
        assert!(shell.child_id() > 0);
        let _ = shell.child.kill();
    }
}
