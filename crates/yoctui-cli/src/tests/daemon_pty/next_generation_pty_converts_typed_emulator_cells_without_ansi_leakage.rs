use super::*;

#[test]
fn next_generation_pty_converts_typed_emulator_cells_without_ansi_leakage() {
    let mut emulator = yoctui_model::TerminalEmulator::new(
        PtyDimensions {
            columns: 12,
            rows: 3,
        },
        8,
    )
    .unwrap();
    emulator
        .process(b"\x1b[?1h\x1b[2J\x1b[1;1H\x1b(0lqqk\x1b(B\r\nprompt")
        .unwrap();
    let terminal = emulator.snapshot(0).unwrap();
    let screen = terminal_to_wire(PtySessionId(3), &terminal);
    assert_eq!(screen.session_id.0, 3);
    assert!(screen.application_cursor);
    for symbol in ["┌", "─", "┐"] {
        assert!(screen.cells.iter().any(|cell| cell.contents == symbol));
    }
    assert!(
        !screen
            .cells
            .iter()
            .any(|cell| matches!(cell.contents.as_str(), "l" | "q" | "k"))
    );
    assert!(
        screen
            .cells
            .iter()
            .all(|cell| !cell.contents.contains('\x1b'))
    );
}
