use super::*;

#[tokio::test]
async fn recovered_pty_identifiers_advance_generic_allocation() {
    let mut supervisor = DaemonPtySupervisor::with_recovered_session_ids([
        1,
        7,
        super::super::RAW_PTY_NAMESPACE | 99,
    ]);

    let session = supervisor
        .start_new(
            "new session".into(),
            PtyKind::Utility,
            "/tmp".into(),
            PtyCommand {
                program: "/bin/true".into(),
                arguments: Vec::new(),
                environment_profile_id: None,
            },
            TerminalDimensions {
                columns: 80,
                rows: 24,
            },
        )
        .unwrap();

    assert_eq!(session, PtySessionId(8));
    supervisor.close(session).unwrap();
    let after_close = supervisor
        .start_new(
            "after close".into(),
            PtyKind::Utility,
            "/tmp".into(),
            PtyCommand {
                program: "/bin/true".into(),
                arguments: Vec::new(),
                environment_profile_id: None,
            },
            TerminalDimensions {
                columns: 80,
                rows: 24,
            },
        )
        .unwrap();
    assert_eq!(after_close, PtySessionId(9));
}
