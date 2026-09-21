use super::*;

fn emulator(rows: u16, columns: u16, scrollback: usize) -> TerminalEmulator {
    TerminalEmulator::new(PtyDimensions { columns, rows }, scrollback).unwrap()
}

mod terminal_emulation_tracks_cursor_styles_unicode_and_modes;

mod ux_terminal_adapter_snapshot_is_complete_bounded_and_parser_owned;

mod terminal_emulation_bounds_and_restores_scrollback_snapshots;
