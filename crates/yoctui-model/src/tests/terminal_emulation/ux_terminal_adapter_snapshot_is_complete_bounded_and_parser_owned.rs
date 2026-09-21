use super::*;

#[test]
fn ux_terminal_adapter_snapshot_is_complete_bounded_and_parser_owned() {
    let mut terminal = emulator(3, 12, 4);
    terminal
        .process(
            b"\x1b[1;1Hplain \x1b[1;3;4;38;2;4;5;6;48;5;17mstyled\x1b[0m\r\n\
                  wide:\xe7\x95\x8c\r\nthird\r\nfourth\x1b[?25l",
        )
        .unwrap();
    let snapshot = terminal.snapshot(1).unwrap();
    assert_eq!(
        snapshot.cells.len(),
        usize::from(snapshot.dimensions.rows) * usize::from(snapshot.dimensions.columns)
    );
    assert!(snapshot.cells.iter().any(|cell| {
        cell.contents == "s"
            && cell.bold
            && cell.italic
            && cell.underline
            && cell.foreground == TerminalColor::Rgb(4, 5, 6)
            && cell.background == TerminalColor::Indexed(17)
    }));
    assert!(snapshot.cells.iter().any(|cell| cell.wide));
    assert!(snapshot.cells.iter().any(|cell| cell.wide_continuation));
    assert!(snapshot.modes.cursor_hidden);
    assert_eq!(snapshot.scrollback_offset, 1);
    assert!(snapshot.max_scrollback_offset >= snapshot.scrollback_offset);
    assert!(
        snapshot
            .cells
            .iter()
            .all(|cell| cell.contents.len() <= MAX_TERMINAL_CELL_BYTES)
    );
}
