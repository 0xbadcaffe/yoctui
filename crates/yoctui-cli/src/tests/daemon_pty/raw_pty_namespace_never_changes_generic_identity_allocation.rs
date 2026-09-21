use super::*;

#[tokio::test]
async fn raw_pty_namespace_never_changes_generic_identity_allocation() {
    let mut supervisor = DaemonPtySupervisor::default();
    let command = PtyCommand {
        program: "/bin/true".into(),
        arguments: Vec::new(),
        environment_profile_id: None,
    };
    let dimensions = TerminalDimensions {
        columns: 80,
        rows: 24,
    };
    supervisor
        .start(
            PtySessionId(6 << 60 | 1),
            "raw namespace fixture".into(),
            PtyKind::Utility,
            "/tmp".into(),
            command.clone(),
            dimensions,
        )
        .unwrap();
    let generic = supervisor
        .start_new(
            "generic".into(),
            PtyKind::Utility,
            "/tmp".into(),
            command,
            dimensions,
        )
        .unwrap();
    assert_eq!(generic, PtySessionId(1));
    assert_eq!(generic.0 >> 60, 0);
}
