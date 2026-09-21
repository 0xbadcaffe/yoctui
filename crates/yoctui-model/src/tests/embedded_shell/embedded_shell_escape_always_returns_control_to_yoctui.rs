use super::*;

#[test]
fn embedded_shell_escape_always_returns_control_to_yoctui() {
    assert!(ShellSession::emergency_escape(0x1d));
    let mut session = ShellSession::new(1, "/", "env");
    session.status = ShellSessionStatus::Running;
    session.mode = ShellInputMode::Copy;
    assert!(!session.owns_input());
}
