use super::*;
use yoctui_model::{
    ClientDaemonLifecycle, ClientDaemonPtyDetails, ClientDaemonPtyKind, ClientDaemonPtySummary,
    ClientReplicaStatus, PtyDimensions, TerminalWorkbenchMode,
};

fn app() -> App {
    let mut app = App::new(16, 4096);
    app.onboarding.open = false;
    app.screen = Screen::TerminalSessions;
    app.focus = FocusTarget::Navigator;
    app.daemon.status = ClientReplicaStatus::Current;
    app.daemon.pty_sessions.push(ClientDaemonPtySummary {
        id: 41,
        name: "GitUI · source".into(),
        lifecycle: ClientDaemonLifecycle::Running,
        viewers: 1,
    });
    app.daemon.pty_details.push(ClientDaemonPtyDetails {
        id: 41,
        kind: ClientDaemonPtyKind::Utility,
        cwd: "/source".into(),
        columns: 120,
        rows: 40,
        writer: None,
        writer_epoch: 7,
        exit_code: None,
        restartable: true,
    });
    app
}

#[test]
fn gitui_native_workspace_mouse_focus_and_writer_keys_use_exact_full_pane() {
    let mut app = app();
    assert_eq!(workbench_pane_widths(&app, 160, 50), [27, 133, 0]);
    assert_eq!(
        terminal_workspace_dimensions(&app, 160, 50),
        Some(PtyDimensions {
            columns: 131,
            rows: 35
        })
    );
    let action = mouse_action_for_app(
        MouseInput {
            kind: MouseKind::Down,
            column: 40,
            row: 20,
        },
        &app,
        160,
        50,
    )
    .unwrap();
    assert!(matches!(action, Action::SelectPtyPane { .. }));
    yoctui_model::update(&mut app, action);
    assert_eq!(app.focus, FocusTarget::Workspace);
    assert_eq!(
        terminal_workspace_action(&app, Input::Char('o')),
        Some(Action::TerminalTakeControl)
    );
    assert!(!terminal_owns_input(&app));
    app.terminal.client_id = Some([1; 16]);
    app.daemon.pty_details[0].writer = Some([1; 16]);
    assert!(terminal_owns_input(&app));
    for input in [Input::Char('q'), Input::Tab, Input::Enter, Input::Esc] {
        assert!(
            terminal_workspace_action(&app, input).is_none(),
            "writer keys remain native, not viewer shortcuts"
        );
    }
    app.daemon.pty_details[0].writer = Some([2; 16]);
    assert!(!terminal_owns_input(&app));
    app.daemon.status = ClientReplicaStatus::Stale;
    assert!(!terminal_owns_input(&app));
    app.terminal.mode = TerminalWorkbenchMode::KillConfirmation;
    assert!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 40,
                row: 20
            },
            &app,
            160,
            50
        )
        .is_none()
    );
}

#[test]
fn gitui_native_geometry_preserves_other_utilities_splits_and_narrow_terminals() {
    let mut app = app();
    app.focus = FocusTarget::Workspace;
    app.daemon.pty_sessions[0].name = "Show machines".into();
    assert_eq!(workbench_pane_widths(&app, 160, 50), [27, 100, 33]);
    assert_eq!(
        terminal_workspace_dimensions(&app, 160, 50),
        Some(PtyDimensions {
            columns: 98,
            rows: 32
        })
    );
    app.daemon.pty_sessions[0].name = "GitUI · devtool busybox".into();
    app.split_terminal_pane(yoctui_model::SplitAxis::Horizontal)
        .unwrap();
    let dimensions = terminal_workspace_dimensions(&app, 160, 50).unwrap();
    assert!(dimensions.columns < 131 && dimensions.rows == 35);
    for (w, h) in [(120, 32), (100, 30), (80, 24), (20, 5), (1, 1)] {
        let _ = terminal_workspace_dimensions(&app, w, h);
    }
}
