use super::*;

#[test]
fn ux_terminal_cli_encodes_exact_pty_bytes_including_literal_prefix() {
    assert_eq!(
        terminal_input_bytes(Input::Char('界')),
        Some("界".as_bytes().to_vec())
    );
    assert_eq!(terminal_input_bytes(Input::Enter), Some(b"\r".to_vec()));
    assert_eq!(terminal_input_bytes(Input::CtrlB), Some(vec![0x02]));
    assert_eq!(terminal_input_bytes(Input::Up), Some(b"\x1b[A".to_vec()));
    assert_eq!(
        terminal_input_bytes(Input::BackTab),
        Some(b"\x1b[Z".to_vec())
    );
    assert_eq!(terminal_input_bytes(Input::F10), Some(b"\x1b[21~".to_vec()));
    assert_eq!(terminal_input_bytes(Input::F12), Some(b"\x1b[24~".to_vec()));
}
