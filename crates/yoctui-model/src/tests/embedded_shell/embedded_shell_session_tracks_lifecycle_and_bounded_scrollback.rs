use super::*;

#[test]
fn embedded_shell_session_tracks_lifecycle_and_bounded_scrollback() {
    let mut session = ShellSession::new(7, "/build", "poky:qemux86-64");
    session.status = ShellSessionStatus::Running;
    assert!(session.owns_input());
    for line in 0..(MAX_SCROLLBACK_LINES + 10) {
        session.push_output(line.to_string());
    }
    assert_eq!(session.scrollback.len(), MAX_SCROLLBACK_LINES);
    assert_eq!(session.scrollback.front().map(String::as_str), Some("10"));
}
