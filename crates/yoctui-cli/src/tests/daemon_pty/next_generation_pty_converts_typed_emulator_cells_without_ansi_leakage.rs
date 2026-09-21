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
        .process(b"\x1b[2J\x1b[1;1Hready\r\nprompt")
        .unwrap();
    let terminal = emulator.snapshot(0).unwrap();
    let screen = terminal_to_wire(PtySessionId(3), &terminal);
    assert_eq!(screen.session_id.0, 3);
    assert!(screen.cells.iter().any(|cell| cell.contents == "r"));
    assert!(
        screen
            .cells
            .iter()
            .all(|cell| !cell.contents.contains('\x1b'))
    );
}
