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

#[test]
fn ux_terminal_runtime_scoped_commands_ignore_unrelated_journal_generations() {
    use yoctui_protocol::daemon::{JobId, PtySessionId};
    for command in [
        DaemonCommand::TerminatePty {
            session_id: PtySessionId(3),
            force: true,
            confirmation: None,
        },
        DaemonCommand::RenamePty {
            session_id: PtySessionId(3),
            name: "shell".into(),
        },
        DaemonCommand::ClosePty {
            session_id: PtySessionId(3),
        },
        DaemonCommand::TakePtyControl {
            session_id: PtySessionId(3),
            expected_epoch: 7,
        },
        DaemonCommand::CancelJob { job_id: JobId(71) },
    ] {
        assert_eq!(command.expected_generation(10), None);
        assert_eq!(command.expected_generation(11), None);
    }
    let app = App::new(16, 4096);
    let mut app = app;
    app.workspace.build_dir = Some("/build".into());
    let command = prefix_daemon_command(&app, PrefixCommand::CreateSession)
        .unwrap()
        .unwrap();
    assert_eq!(command.expected_generation(10), None);
    for command in [
        DaemonCommand::PrepareShutdown,
        DaemonCommand::StartBuild {
            targets: vec!["image".into()],
            task: None,
            force: false,
        },
    ] {
        assert_eq!(command.expected_generation(10), Some(10));
    }
}
