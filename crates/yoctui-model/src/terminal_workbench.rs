//! Bounded client-local presentation state for daemon-owned terminal sessions.

use std::path::PathBuf;

pub const MAX_TERMINAL_SEARCH_BYTES: usize = 256;
pub const MAX_TERMINAL_RENAME_BYTES: usize = 128;
pub const MAX_TERMINAL_PASTE_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TerminalWorkbenchMode {
    #[default]
    Live,
    Copy,
    Search,
    Rename,
    PasteReview,
    KillConfirmation,
    Help,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TerminalWorkbenchState {
    pub mode: TerminalWorkbenchMode,
    pub query: String,
    pub rename: String,
    pub pending_paste: Vec<u8>,
    pub scrollback_offset: usize,
    pub copy_row: usize,
    pub client_id: Option<[u8; 16]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalEffect {
    Create {
        name: String,
        kind: TerminalCreationKind,
        cwd: PathBuf,
        program: PathBuf,
        arguments: Vec<String>,
    },
    TakeControl {
        session_id: u64,
        expected_epoch: u64,
    },
    ReleaseControl {
        session_id: u64,
        writer_epoch: u64,
    },
    Input {
        session_id: u64,
        writer_epoch: u64,
        bytes: Vec<u8>,
    },
    Viewport {
        session_id: u64,
        scrollback_offset: usize,
    },
    Rename {
        session_id: u64,
        name: String,
    },
    Terminate {
        session_id: u64,
    },
    Close {
        session_id: u64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalCreationKind {
    BuildShell,
    Devshell,
    Menuconfig,
}

impl TerminalWorkbenchState {
    pub fn reset_transient_mode(&mut self) {
        self.mode = TerminalWorkbenchMode::Live;
        self.rename.clear();
        self.pending_paste.clear();
    }

    pub fn append_query(&mut self, character: char) -> bool {
        append_bounded(&mut self.query, character, MAX_TERMINAL_SEARCH_BYTES)
    }

    pub fn backspace_query(&mut self) {
        self.query.pop();
    }

    pub fn append_rename(&mut self, character: char) -> bool {
        !character.is_control()
            && append_bounded(&mut self.rename, character, MAX_TERMINAL_RENAME_BYTES)
    }

    pub fn backspace_rename(&mut self) {
        self.rename.pop();
    }

    pub fn stage_paste(&mut self, text: &str) -> bool {
        if text.is_empty() || text.len() > MAX_TERMINAL_PASTE_BYTES {
            return false;
        }
        self.pending_paste.clear();
        self.pending_paste.extend_from_slice(text.as_bytes());
        self.mode = TerminalWorkbenchMode::PasteReview;
        true
    }

    pub fn set_scrollback_offset(&mut self, offset: usize, retained_lines: usize) {
        self.scrollback_offset = offset.min(retained_lines);
        self.copy_row = self.copy_row.min(retained_lines);
    }
}

fn append_bounded(value: &mut String, character: char, maximum: usize) -> bool {
    let next = value.len().saturating_add(character.len_utf8());
    if next > maximum {
        return false;
    }
    value.push(character);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ux_terminal_workbench_bounds_search_rename_paste_and_scrollback() {
        let mut state = TerminalWorkbenchState::default();
        for _ in 0..MAX_TERMINAL_SEARCH_BYTES {
            assert!(state.append_query('x'));
        }
        assert!(!state.append_query('x'));
        assert!(!state.append_rename('\n'));
        assert!(!state.stage_paste(&"x".repeat(MAX_TERMINAL_PASTE_BYTES + 1)));
        assert!(state.stage_paste("printf 'safe review'\n"));
        assert_eq!(state.mode, TerminalWorkbenchMode::PasteReview);
        state.set_scrollback_offset(500, 42);
        assert_eq!(state.scrollback_offset, 42);
        state.reset_transient_mode();
        assert!(state.pending_paste.is_empty());
    }
}
