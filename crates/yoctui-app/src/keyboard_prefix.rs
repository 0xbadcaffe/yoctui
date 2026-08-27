use std::time::{Duration, Instant};

use crate::Input;

pub const DEFAULT_PREFIX_TIMEOUT: Duration = Duration::from_millis(1_000);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixCommand {
    CreateSession,
    NextSession,
    PreviousSession,
    SplitHorizontal,
    SplitVertical,
    ClosePane,
    Detach,
    CommandPalette,
    Help,
    TakeControl,
    OpenTerminalSessions,
    CopyMode,
    Search,
    Rename,
    ReleaseControl,
    Kill,
    Zoom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrefixEvent {
    Awaiting,
    Command(PrefixCommand),
    Literal(Input),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrefixState {
    pending: bool,
    deadline: Option<Instant>,
    timeout: Duration,
}

impl Default for PrefixState {
    fn default() -> Self {
        Self::new(DEFAULT_PREFIX_TIMEOUT)
    }
}

impl PrefixState {
    pub const fn new(timeout: Duration) -> Self {
        Self {
            pending: false,
            deadline: None,
            timeout,
        }
    }

    pub fn pending(&self) -> bool {
        self.pending
    }

    pub fn feed(&mut self, input: Input, now: Instant) -> PrefixEvent {
        if self.pending && self.deadline.is_some_and(|deadline| now >= deadline) {
            self.reset();
        }
        if !self.pending {
            if input == Input::CtrlB {
                self.pending = true;
                self.deadline = Some(now + self.timeout);
                PrefixEvent::Awaiting
            } else {
                PrefixEvent::Literal(input)
            }
        } else {
            self.reset();
            match input {
                Input::Char('c') => PrefixEvent::Command(PrefixCommand::CreateSession),
                Input::Char('n') => PrefixEvent::Command(PrefixCommand::NextSession),
                Input::Char('p') => PrefixEvent::Command(PrefixCommand::PreviousSession),
                Input::Char('%') => PrefixEvent::Command(PrefixCommand::SplitHorizontal),
                Input::Char('"') => PrefixEvent::Command(PrefixCommand::SplitVertical),
                Input::Char('x') => PrefixEvent::Command(PrefixCommand::ClosePane),
                Input::Char('d') => PrefixEvent::Command(PrefixCommand::Detach),
                Input::Char(':') => PrefixEvent::Command(PrefixCommand::CommandPalette),
                Input::Char('?') => PrefixEvent::Command(PrefixCommand::Help),
                Input::Char('o') => PrefixEvent::Command(PrefixCommand::TakeControl),
                Input::Char('t') => PrefixEvent::Command(PrefixCommand::OpenTerminalSessions),
                Input::Char('[') => PrefixEvent::Command(PrefixCommand::CopyMode),
                Input::Char('/') => PrefixEvent::Command(PrefixCommand::Search),
                Input::Char('r') => PrefixEvent::Command(PrefixCommand::Rename),
                Input::Char('O') => PrefixEvent::Command(PrefixCommand::ReleaseControl),
                Input::Char('K') => PrefixEvent::Command(PrefixCommand::Kill),
                Input::Char('z') => PrefixEvent::Command(PrefixCommand::Zoom),
                // A second prefix sends a literal Ctrl+B to the terminal.
                Input::CtrlB => PrefixEvent::Literal(Input::CtrlB),
                other => PrefixEvent::Literal(other),
            }
        }
    }

    pub fn reset(&mut self) {
        self.pending = false;
        self.deadline = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_requires_a_second_key_and_resets_after_command() {
        let now = Instant::now();
        let mut state = PrefixState::new(Duration::from_secs(1));
        assert_eq!(state.feed(Input::CtrlB, now), PrefixEvent::Awaiting);
        assert!(state.pending());
        assert_eq!(
            state.feed(Input::Char('n'), now),
            PrefixEvent::Command(PrefixCommand::NextSession)
        );
        assert!(!state.pending());
    }

    #[test]
    fn prefix_timeout_returns_next_key_to_the_application() {
        let now = Instant::now();
        let mut state = PrefixState::new(Duration::from_millis(10));
        assert_eq!(state.feed(Input::CtrlB, now), PrefixEvent::Awaiting);
        assert_eq!(
            state.feed(Input::Char('x'), now + Duration::from_millis(11)),
            PrefixEvent::Literal(Input::Char('x'))
        );
        assert!(!state.pending());
    }

    #[test]
    fn double_prefix_is_a_literal_control_b() {
        let now = Instant::now();
        let mut state = PrefixState::default();
        state.feed(Input::CtrlB, now);
        assert_eq!(
            state.feed(Input::CtrlB, now),
            PrefixEvent::Literal(Input::CtrlB)
        );
    }

    #[test]
    fn ux_terminal_prefix_opens_the_terminal_workbench() {
        let now = Instant::now();
        let mut state = PrefixState::default();
        state.feed(Input::CtrlB, now);
        assert_eq!(
            state.feed(Input::Char('t'), now),
            PrefixEvent::Command(PrefixCommand::OpenTerminalSessions)
        );
        for (input, command) in [
            (Input::Char('['), PrefixCommand::CopyMode),
            (Input::Char('/'), PrefixCommand::Search),
            (Input::Char('r'), PrefixCommand::Rename),
            (Input::Char('O'), PrefixCommand::ReleaseControl),
            (Input::Char('K'), PrefixCommand::Kill),
            (Input::Char('z'), PrefixCommand::Zoom),
        ] {
            state.feed(Input::CtrlB, now);
            assert_eq!(state.feed(input, now), PrefixEvent::Command(command));
        }
    }
}
