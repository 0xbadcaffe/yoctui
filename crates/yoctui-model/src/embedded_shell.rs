use std::collections::VecDeque;

pub const SHELL_ESCAPE_CHORD: &str = "Ctrl+]";
pub const MAX_SHELL_SESSIONS: usize = 4;
const MAX_SCROLLBACK_LINES: usize = 4000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellSessionStatus {
    Starting,
    Running,
    Exited,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellInputMode {
    Foreground,
    Copy,
    Search,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellSession {
    pub id: u64,
    pub cwd: String,
    pub environment_identity: String,
    pub status: ShellSessionStatus,
    pub mode: ShellInputMode,
    pub exit_status: Option<i32>,
    pub unread_activity: bool,
    pub scrollback: VecDeque<String>,
}

impl ShellSession {
    pub fn new(id: u64, cwd: impl Into<String>, environment_identity: impl Into<String>) -> Self {
        Self {
            id,
            cwd: cwd.into(),
            environment_identity: environment_identity.into(),
            status: ShellSessionStatus::Starting,
            mode: ShellInputMode::Foreground,
            exit_status: None,
            unread_activity: false,
            scrollback: VecDeque::new(),
        }
    }
    pub fn push_output(&mut self, line: impl Into<String>) {
        if self.scrollback.len() >= MAX_SCROLLBACK_LINES {
            self.scrollback.pop_front();
        }
        self.scrollback.push_back(line.into());
        self.unread_activity = true;
    }
    pub fn owns_input(&self) -> bool {
        self.mode == ShellInputMode::Foreground && self.status == ShellSessionStatus::Running
    }
    pub fn emergency_escape(byte: u8) -> bool {
        byte == 0x1d
    }
}

#[cfg(test)]
#[path = "tests/embedded_shell/mod.rs"]
mod tests;
