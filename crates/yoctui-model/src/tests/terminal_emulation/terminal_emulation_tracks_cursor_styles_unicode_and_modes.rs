use super::*;

#[test]
fn terminal_emulation_tracks_cursor_styles_unicode_and_modes() {
    let mut terminal = emulator(6, 30, 20);
    terminal
        .process(
            b"plain \x1b[1;3;4;38;2;1;2;3;48;5;17mstyled\x1b[0m\r\n\
                  \x1b[?1049h\x1b[2J\x1b[3;5Hmenu \xe2\x94\x82\x1b[?25l\
                  \x1b[?1h\x1b=\x1b[?2004h\x1b[?1002h\x1b[?1006h",
        )
        .unwrap();
    let snapshot = terminal.snapshot(0).unwrap();
    assert!(snapshot.modes.alternate_screen);
    assert!(snapshot.modes.cursor_hidden);
    assert!(snapshot.modes.application_cursor);
    assert!(snapshot.modes.application_keypad);
    assert!(snapshot.modes.bracketed_paste);
    assert_eq!(snapshot.modes.mouse, TerminalMouseMode::ButtonMotion);
    assert_eq!(snapshot.modes.mouse_encoding, TerminalMouseEncoding::Sgr);
    assert!(snapshot.plain_text.contains("menu │"));
    assert_eq!(snapshot.cursor, (2, 10));

    terminal.process(b"\x1b[?1049l").unwrap();
    let snapshot = terminal.snapshot(0).unwrap();
    let styled = &snapshot.cells[6];
    assert_eq!(styled.contents, "s");
    assert!(styled.bold && styled.italic && styled.underline);
    assert_eq!(styled.foreground, TerminalColor::Rgb(1, 2, 3));
    assert_eq!(styled.background, TerminalColor::Indexed(17));
}
