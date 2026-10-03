use super::*;
use yoctui_model::{ClientDaemonLifecycle as Life, ClientReplicaStatus};

fn terminal_fixture() -> App {
    let mut app = App::new(8, 1_000);
    app.screen = Screen::TerminalSessions;
    app.focus = FocusTarget::Workspace;
    app.daemon.status = ClientReplicaStatus::Current;
    app.terminal.client_id = Some([3; 16]);
    app.daemon
        .pty_sessions
        .push(yoctui_model::ClientDaemonPtySummary {
            id: 1,
            name: "shell".into(),
            lifecycle: Life::Running,
            viewers: 1,
        });
    app.daemon
        .pty_details
        .push(yoctui_model::ClientDaemonPtyDetails {
            id: 1,
            kind: yoctui_model::ClientDaemonPtyKind::BuildShell,
            cwd: "/build".into(),
            columns: 80,
            rows: 24,
            writer: Some([3; 16]),
            writer_epoch: 9,
            exit_code: None,
            restartable: true,
        });
    app
}

#[test]
fn terminal_exit_renders_read_only_history_not_an_active_writer_offer() {
    let mut app = terminal_fixture();
    for lifecycle in [
        Life::Disconnected,
        Life::Connecting,
        Life::Stopping,
        Life::Exited,
        Life::Failed,
        Life::Lost,
    ] {
        app.daemon.pty_sessions[0].lifecycle = lifecycle;
        for writer in [Some([3; 16]), Some([4; 16]), None] {
            app.daemon.pty_details[0].writer = writer;
            let inspector = terminal_workspace::terminal_session_inspector_text(&app);
            assert!(
                inspector.contains("Read-only history (no active writer)"),
                "{inspector}"
            );
            assert!(!inspector.contains("another client owns writer"));
            assert!(!inspector.contains("writer lease available"));
            assert!(inspector.contains("epoch 9"));
            let output = rendered_region_rows(160, 30, |frame, area| {
                terminal_workspace::terminal_sessions_workspace(frame, &app, area);
            })
            .join("\n");
            assert!(output.contains("READ-ONLY · no active writer"), "{output}");
            assert!(!output.contains("writer held by another client"));
            assert!(!output.contains("writer available"));
            assert!(!output.contains("BuildShell · writer ·"), "{output}");
            assert!(
                output.contains("BuildShell · read-only history ·"),
                "{output}"
            );
            for (width, height) in [(20, 8), (80, 24), (160, 50)] {
                let _ = rendered_text_at(&app, width, height, literal_now());
            }
        }
    }
}

#[test]
fn terminal_live_writer_viewer_and_stale_roles_remain_distinct() {
    let mut app = terminal_fixture();
    assert!(
        terminal_workspace::terminal_session_inspector_text(&app)
            .contains("keyboard/paste/resize enabled")
    );
    app.daemon.pty_details[0].writer = Some([4; 16]);
    assert!(
        terminal_workspace::terminal_session_inspector_text(&app)
            .contains("another client owns writer")
    );
    app.daemon.pty_details[0].writer = None;
    assert!(
        terminal_workspace::terminal_session_inspector_text(&app)
            .contains("writer lease available")
    );
    app.daemon.pty_details[0].writer = Some([3; 16]);
    for status in [
        ClientReplicaStatus::Stale,
        ClientReplicaStatus::Synchronizing,
        ClientReplicaStatus::Disconnected,
    ] {
        app.daemon.status = status;
        let inspector = terminal_workspace::terminal_session_inspector_text(&app);
        assert!(inspector.contains("Retained read-only (reconnect for control)"));
        assert!(!inspector.contains("enabled"));
        assert!(!inspector.contains("another client owns writer"));
    }
}

#[test]
fn terminal_split_roles_use_each_panes_own_lifecycle() {
    let mut app = terminal_fixture();
    let mut ended = app.daemon.pty_sessions[0].clone();
    ended.id = 2;
    ended.name = "ended-gdb".into();
    ended.lifecycle = Life::Exited;
    app.daemon.pty_sessions.push(ended);
    let mut details = app.daemon.pty_details[0].clone();
    details.id = 2;
    app.daemon.pty_details.push(details);
    let root = app.pane_layout.focused;
    app.pane_layout
        .split(root, yoctui_model::SplitAxis::Horizontal)
        .unwrap();
    assert!(app.selected_terminal_is_writer());
    let output = rendered_region_rows(200, 30, |frame, area| {
        terminal_workspace::terminal_sessions_workspace(frame, &app, area);
    })
    .join("\n");
    assert!(output.contains("BuildShell · writer ·"), "{output}");
    assert!(
        output.contains("BuildShell · read-only history ·"),
        "{output}"
    );
    assert!(output.contains("ended-gdb"));
    assert_eq!(app.daemon.pty_details[1].writer, Some([3; 16]));
    assert_eq!(app.daemon.pty_sessions[1].lifecycle, Life::Exited);
}
