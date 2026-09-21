use super::*;

#[test]
fn ux_terminal_adapter_wire_is_sparse_exact_and_never_reparses_ansi() {
    let mut emulator = yoctui_model::TerminalEmulator::new(
        PtyDimensions {
            columns: 16,
            rows: 3,
        },
        8,
    )
    .unwrap();
    emulator
        .process(b"\x1b[1;3;4;38;2;7;8;9mstyled\x1b[0m\r\nwide:\xe7\x95\x8c\x1b[?25l")
        .unwrap();
    let terminal = emulator.snapshot(0).unwrap();
    let screen = terminal_to_wire(PtySessionId(4), &terminal);
    assert!(screen.cursor_hidden);
    assert!(screen.cells.len() < terminal.cells.len());
    assert!(
        screen
            .cells
            .windows(2)
            .all(|pair| pair[0].index < pair[1].index)
    );
    assert!(screen.cells.iter().any(|cell| {
        cell.contents == "s"
            && cell.bold
            && cell.italic
            && cell.underline
            && cell.foreground == yoctui_protocol::daemon::PtyTerminalColor::Rgb(7, 8, 9)
    }));
    assert!(screen.cells.iter().any(|cell| cell.wide));
    assert!(screen.cells.iter().any(|cell| cell.wide_continuation));
    assert!(
        screen
            .cells
            .iter()
            .all(|cell| !cell.contents.contains('\x1b'))
    );
}
