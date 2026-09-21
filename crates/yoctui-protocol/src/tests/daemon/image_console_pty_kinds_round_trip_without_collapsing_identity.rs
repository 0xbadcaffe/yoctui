use super::*;

#[test]
fn image_console_pty_kinds_round_trip_without_collapsing_identity() {
    for kind in [PtyKind::QemuConsole, PtyKind::SshConsole] {
        let summary = PtySessionSummary {
            id: PtySessionId(41),
            name: "image console".into(),
            kind,
            cwd: "/build".into(),
            lifecycle: LifecycleState::Running,
            dimensions: TerminalDimensions {
                columns: 120,
                rows: 40,
            },
            writer: None,
            writer_epoch: 0,
            viewers: 1,
            exit_code: None,
            restartable: true,
        };
        let encoded = serde_json::to_vec(&summary).unwrap();
        let decoded: PtySessionSummary = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, summary);
        let text = String::from_utf8(encoded).unwrap();
        assert!(
            text.contains(match kind {
                PtyKind::QemuConsole => "qemu_console",
                PtyKind::SshConsole => "ssh_console",
                _ => unreachable!(),
            }),
            "{text}"
        );
    }
}
