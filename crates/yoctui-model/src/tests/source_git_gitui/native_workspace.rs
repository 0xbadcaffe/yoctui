use super::*;

fn app() -> App {
    let mut app = App::new(8, 1024);
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
fn gitui_native_layout_hint_is_metadata_only_and_never_writer_authority() {
    let mut app = app();
    for name in [
        "GitUI · source",
        "GitUI · editor layer",
        "GitUI · devtool busybox",
    ] {
        app.daemon.pty_sessions[0].name = name.into();
        assert!(app.selected_terminal_is_gitui());
        assert!(app.selected_terminal_uses_native_workspace());
        assert!(!app.selected_terminal_is_writer());
    }
    for name in ["GitUI", "GitUI · editor ", "Show machines", "renamed"] {
        app.daemon.pty_sessions[0].name = name.into();
        assert!(!app.selected_terminal_is_gitui());
    }
    app.daemon.pty_sessions[0].name = "GitUI · source".into();
    app.daemon.pty_details[0].kind = ClientDaemonPtyKind::BuildShell;
    assert!(!app.selected_terminal_is_gitui());
    app.daemon.pty_details[0].kind = ClientDaemonPtyKind::Menuconfig;
    assert!(app.selected_terminal_uses_native_workspace());
}

#[test]
fn gitui_pane_click_focuses_native_input_without_taking_a_writer_lease() {
    let mut app = app();
    let pane = app.pane_layout.focused;
    assert!(update(&mut app, Action::SelectPtyPane { pane, index: 0 }).is_none());
    assert_eq!(app.focus, FocusTarget::Workspace);
    assert!(!app.selected_terminal_is_writer());
    app.focus = FocusTarget::Navigator;
    update(&mut app, Action::SelectPtyPane { pane, index: 99 });
    assert_eq!(app.focus, FocusTarget::Navigator);
    app.focus = FocusTarget::Workspace;
    assert!(matches!(
        update(&mut app, Action::TerminalTakeControl),
        Some(Effect::Terminal(TerminalEffect::TakeControl {
            session_id: 41,
            expected_epoch: 7
        }))
    ));
    assert!(
        !app.selected_terminal_is_writer(),
        "Only the daemon grant can enable writer input"
    );
}
