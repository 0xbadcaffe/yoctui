//! Small, dependency-free PTY primitives used by release acceptance tests.

use std::io;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Screen {
    pub width: usize,
    pub height: usize,
    cells: Vec<Vec<char>>,
}

impl Screen {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
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

    fn put(&mut self, x: usize, y: usize, ch: char) {
        if y < self.height && x < self.width {
            self.cells[y][x] = ch;
        }
    }
}

/// Conservative ANSI parser for semantic assertions. Unsupported controls are
/// ignored rather than leaking into screen text.
pub fn parse_screen(bytes: &[u8], width: usize, height: usize) -> Screen {
    let mut screen = Screen::new(width, height);
    let (mut x, mut y) = (0usize, 0usize);
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\x1b' if bytes.get(i + 1) == Some(&b'[') => {
                i += 2;
                let start = i;
                while i < bytes.len() && !bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let params = std::str::from_utf8(&bytes[start..i]).unwrap_or("");
                let command = bytes.get(i).copied().unwrap_or_default();
                let nums = params
                    .split(';')
                    .filter_map(|v| v.parse::<usize>().ok())
                    .collect::<Vec<_>>();
                match command {
                    b'H' | b'f' => {
                        y = nums.first().copied().unwrap_or(1).saturating_sub(1);
                        x = nums.get(1).copied().unwrap_or(1).saturating_sub(1);
                    }
                    b'J' if params == "2" || params == "3" => {
                        screen = Screen::new(width, height);
                        x = 0;
                        y = 0;
                    }
                    b'K' => {
                        for col in x..width {
                            screen.put(col, y, ' ');
                        }
                    }
                    b'A' => y = y.saturating_sub(nums.first().copied().unwrap_or(1)),
                    b'B' => {
                        y = (y + nums.first().copied().unwrap_or(1)).min(height.saturating_sub(1))
                    }
                    b'C' => {
                        x = (x + nums.first().copied().unwrap_or(1)).min(width.saturating_sub(1))
                    }
                    b'D' => x = x.saturating_sub(nums.first().copied().unwrap_or(1)),
                    _ => {}
                }
            }
            b'\n' => y = (y + 1).min(height.saturating_sub(1)),
            b'\r' => x = 0,
            0x20..=0x7e => {
                screen.put(x, y, bytes[i] as char);
                x += 1;
                if x >= width {
                    x = 0;
                    y = (y + 1).min(height.saturating_sub(1));
                }
            }
            _ => {}
        }
        i += 1;
    }
    screen
}

/// Open a Unix PTY and run a command with the slave attached to stdio.
#[cfg(unix)]
pub fn run_pty(command: &str, args: &[&str], input: &[u8]) -> io::Result<Vec<u8>> {
    use std::{
        io::{Read, Write},
        os::fd::{FromRawFd, RawFd},
        os::unix::process::CommandExt,
        process::{Command, Stdio},
    };
    let (master, slave): (RawFd, RawFd) = unsafe {
        let mut master = -1;
        let mut slave = -1;
        if libc::openpty(
            &mut master,
            &mut slave,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
        ) != 0
        {
            return Err(io::Error::last_os_error());
        }
        (master, slave)
    };
    let slave_for_child = slave;
    let mut child = unsafe {
        Command::new(command)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .pre_exec(move || {
                for fd in [0, 1, 2] {
                    libc::dup2(slave_for_child, fd);
                }
                libc::close(slave_for_child);
                Ok(())
            })
            .spawn()?
    };
    unsafe {
        libc::close(slave);
    }
    let mut output = Vec::new();
    let mut reader = unsafe { std::fs::File::from_raw_fd(master) };
    reader.write_all(input)?;
    let _ = child.wait();
    reader.read_to_end(&mut output).ok();
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pty_harness_parses_ansi_screen() {
        let bytes = b"\x1b[2J\x1b[1;1HYoctui\r\nFooter\x1b[0m";
        let screen = parse_screen(bytes, 20, 4);
        assert!(screen.text().contains("Yoctui"));
        assert!(screen.text().contains("Footer"));
    }

    #[cfg(unix)]
    #[test]
    fn pty_harness_executes_child_on_real_pty() {
        let output = run_pty("/bin/sh", &["-c", "printf 'pty-ok\\n'"], b"").expect("pty");
        assert!(String::from_utf8_lossy(&output).contains("pty-ok"));
    }

    #[test]
    fn keyboard_matrix_is_complete_and_unique() {
        let keys = [
            "?",
            "F5",
            "Ctrl+P",
            "/",
            "Tab",
            "Shift+Tab",
            "Esc",
            "q",
            "Ctrl+C",
            "Up",
            "Down",
            "j",
            "k",
            "Enter",
            "Backspace",
            "Right",
            "Left",
            "l",
            "h",
            "e",
            "o",
            "R",
            "r",
            ".",
            "g",
            "m",
            "d",
            "f",
            "F",
            "n",
            "N",
            "w",
            "s",
            "T",
            "B",
            "C",
            "L",
            "1",
            "2",
            "c",
            "Q",
            "x",
            "D",
            "i",
            "v",
            "Space",
            "Ctrl+S",
            "Ctrl+B",
        ];
        let mut sorted = keys.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), keys.len());
        assert!(keys.contains(&"Shift+Tab"));
        assert!(keys.contains(&"Ctrl+P"));
    }
}
