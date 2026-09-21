use std::{
    io,
    os::fd::FromRawFd,
    os::unix::process::CommandExt,
    path::Path,
    process::{Child, Command, Stdio},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminalEmulator {
    pub width: usize,
    pub height: usize,
    pub cursor: (usize, usize),
    cells: Vec<Vec<char>>,
}

impl TerminalEmulator {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            cursor: (0, 0),
            cells: vec![vec![' '; width]; height],
        }
    }
    pub fn text(&self) -> String {
        self.cells
            .iter()
            .map(|row| row.iter().collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }
    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.cells.resize_with(height, || vec![' '; width]);
        for row in &mut self.cells {
            row.resize(width, ' ');
        }
        self.cursor.0 = self.cursor.0.min(width.saturating_sub(1));
        self.cursor.1 = self.cursor.1.min(height.saturating_sub(1));
    }
    pub fn feed(&mut self, bytes: &[u8]) {
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'\x1b' if bytes.get(i + 1) == Some(&b'[') => {
                    i += 2;
                    while i < bytes.len() && !yoctui_utils::is_csi_final_byte(bytes[i]) {
                        i += 1;
                    }
                    if i < bytes.len() && bytes[i] == b'J' {
                        self.cells = vec![vec![' '; self.width]; self.height];
                        self.cursor = (0, 0);
                    }
                }
                b'\r' => self.cursor.0 = 0,
                b'\n' => self.cursor.1 = (self.cursor.1 + 1).min(self.height.saturating_sub(1)),
                0x20..=0x7e => {
                    let (x, y) = self.cursor;
                    if y < self.height && x < self.width {
                        self.cells[y][x] = bytes[i] as char;
                    }
                    self.cursor.0 += 1;
                    if self.cursor.0 >= self.width {
                        self.cursor.0 = 0;
                        self.cursor.1 = (self.cursor.1 + 1).min(self.height.saturating_sub(1));
                    }
                }
                _ => {}
            }
            i += 1;
        }
    }
}

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
#[path = "tests/terminal_emulator.rs"]
mod tests;
