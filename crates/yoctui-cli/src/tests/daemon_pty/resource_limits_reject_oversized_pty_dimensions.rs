use super::*;

#[test]
fn resource_limits_reject_oversized_pty_dimensions() {
    let command = PtyCommand {
        program: "/bin/sh".into(),
        arguments: Vec::new(),
        environment_profile_id: None,
    };
    let dimensions = TerminalDimensions {
        columns: 513,
        rows: 24,
    };
    assert!(
        wire_spec(
            PtySessionId(1),
            "bounded".into(),
            PtyKind::BuildShell,
            "/tmp".into(),
            command,
            dimensions,
        )
        .is_err()
    );
}
