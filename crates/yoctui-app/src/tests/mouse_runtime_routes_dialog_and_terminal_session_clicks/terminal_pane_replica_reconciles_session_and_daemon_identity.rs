use super::*;

#[test]
fn terminal_pane_replica_reconciles_session_and_daemon_identity() {
    use yoctui_protocol::daemon::{
        DaemonInstanceId, LifecycleState, PtyKind, PtySessionId, PtySessionSummary,
        TerminalDimensions,
    };
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([4; 16]),
        123,
        "boot-id".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = daemon_protocol_snapshot(&state);
    for id in [26, 27] {
        snapshot.pty_sessions.push(PtySessionSummary {
            id: PtySessionId(id),
            name: format!("shell-{id}"),
            kind: PtyKind::BuildShell,
            cwd: "/build".into(),
            lifecycle: LifecycleState::Running,
            dimensions: TerminalDimensions {
                columns: 80,
                rows: 24,
            },
            writer: None,
            writer_epoch: 0,
            viewers: 1,
            exit_code: None,
            restartable: true,
        });
    }
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = Screen::TerminalSessions;
    let mut client = DaemonClientSnapshot::default();
    client.replace_app(&mut app, snapshot.clone());
    app.pty_selection = 1;
    let first = app.pane_layout.focused;
    app.split_terminal_pane(yoctui_model::SplitAxis::Horizontal)
        .unwrap();
    app.select_terminal_session(-1);
    snapshot.pty_sessions.reverse();
    client.replace_app(&mut app, snapshot.clone());
    assert_eq!(app.selected_terminal_session().unwrap().id, 26);
    assert_eq!(app.terminal_pane_session_index(first, 0, 2), Some(0));
    snapshot.pty_sessions.retain(|session| session.id.0 != 27);
    client.replace_app(&mut app, snapshot.clone());
    assert_eq!(app.terminal_pane_session_index(first, 0, 2), None);
    assert_eq!(app.selected_terminal_session().unwrap().id, 26);
    snapshot.daemon_instance_id = DaemonInstanceId([8; 16]);
    client.replace_app(&mut app, snapshot);
    assert_eq!(app.selected_terminal_session(), None);
    assert!(!app.selected_terminal_is_writer());
    assert!(
        app.terminal
            .pane_sessions
            .iter()
            .all(|(_, id)| id.is_none())
    );
    assert_eq!(app.screen, Screen::TerminalSessions);
}
