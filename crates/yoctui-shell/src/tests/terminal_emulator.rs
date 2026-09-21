use super::*;
#[test]
fn pty_backend_spawns_real_shell_and_propagates_resize() {
    let mut shell = PtyShell::spawn(Path::new("/bin/sh"), Path::new("/tmp")).unwrap();
    shell.resize(80, 24).unwrap();
    assert!(shell.child_id() > 0);
    let _ = shell.child.kill();
}

#[test]
fn punctuation_csi_keeps_following_text() {
    let mut terminal = TerminalEmulator::new(20, 2);
    terminal.feed(b"a\x1b[1~after");
    assert!(terminal.text().starts_with("aafter"));
}

#[test]
fn terminal_emulation_handles_cursor_clear_resize_and_unicode_safely() {
    let mut terminal = TerminalEmulator::new(20, 4);
    terminal.feed(b"\x1b[2JYoctui\r\nShell");
    assert!(terminal.text().contains("Yoctui"));
    assert!(terminal.text().contains("Shell"));
    terminal.resize(8, 2);
    assert_eq!(terminal.width, 8);
    terminal.feed("é".as_bytes());
    assert!(terminal.cursor.0 <= 8);
}
