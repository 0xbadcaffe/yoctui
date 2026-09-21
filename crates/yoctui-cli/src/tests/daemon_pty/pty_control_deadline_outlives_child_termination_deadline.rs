use super::*;

#[test]
fn pty_control_deadline_outlives_child_termination_deadline() {
    assert!(PTY_CONTROL_RESPONSE_TIMEOUT > PTY_TERMINATION_TIMEOUT);
}
