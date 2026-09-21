use super::*;

#[test]
fn ux_terminal_runtime_prefix_maps_create_and_writer_commands() {
    let mut app = App::new(16, 4096);
    app.workspace.build_dir = Some("/build".into());
    let Some(DaemonCommand::CreatePty { cwd, .. }) =
        prefix_daemon_command(&app, PrefixCommand::CreateSession).unwrap()
    else {
        panic!("expected typed PTY create command");
    };
    assert_eq!(cwd, "/build");
    app.daemon
        .pty_sessions
        .push(yoctui_model::ClientDaemonPtySummary {
            id: 3,
            name: "shell".into(),
            lifecycle: ClientDaemonLifecycle::Running,
            viewers: 1,
        });
    app.daemon
        .pty_details
        .push(yoctui_model::ClientDaemonPtyDetails {
            id: 3,
            kind: yoctui_model::ClientDaemonPtyKind::BuildShell,
            cwd: "/build".into(),
            columns: 120,
            rows: 40,
            writer: None,
            writer_epoch: 7,
            exit_code: None,
            restartable: true,
        });
    assert!(matches!(
        prefix_daemon_command(&app, PrefixCommand::TakeControl).unwrap(),
        Some(DaemonCommand::TakePtyControl { session_id, expected_epoch: 7 })
            if session_id.0 == 3
    ));
    assert!(
        prefix_daemon_command(&app, PrefixCommand::SplitHorizontal)
            .unwrap()
            .is_none()
    );
}
