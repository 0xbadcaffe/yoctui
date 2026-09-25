use super::*;

#[test]
fn focus_menu_shortcut_preserves_editor_and_search_text() {
    let mut app = App::new(10, 1024);
    app.screen = Screen::Recipes;
    app.focus = yoctui_model::FocusTarget::Workspace;
    app.metadata_searching = true;
    assert!(direct_menu_shortcut_action(&app, Input::Char('a'), false).is_none());
    assert!(pane_focus_route(&app, Input::Char('q')).is_none());
    assert_eq!(
        direct_menu_shortcut_action(&app, Input::F12, false),
        Some(Action::OpenApplicationMenu)
    );
}

#[test]
fn function_navigation_remains_global_while_an_embedded_menuconfig_owns_input() {
    let mut app = App::new(8, 1_000);
    app.screen = Screen::Kernel;
    app.focus = yoctui_model::FocusTarget::Workspace;
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.terminal.client_id = Some([4; 16]);
    app.daemon
        .pty_sessions
        .push(yoctui_model::ClientDaemonPtySummary {
            id: 9,
            name: "kernel menuconfig".into(),
            lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
            viewers: 1,
        });
    app.daemon
        .pty_details
        .push(yoctui_model::ClientDaemonPtyDetails {
            id: 9,
            kind: yoctui_model::ClientDaemonPtyKind::Menuconfig,
            cwd: "/build".into(),
            columns: 80,
            rows: 24,
            writer: Some([4; 16]),
            writer_epoch: 1,
            exit_code: None,
            restartable: true,
        });
    app.kernel.menuconfig_terminal = yoctui_model::PlatformTerminalState {
        name: Some("kernel menuconfig".into()),
        session_id: Some(9),
        ..yoctui_model::PlatformTerminalState::default()
    };

    assert!(yoctui_app::terminal_owns_input(&app));
    assert_eq!(
        direct_menu_shortcut_action(&app, Input::F2, false),
        Some(Action::Open(Screen::Tasks))
    );
    assert_eq!(
        direct_menu_shortcut_action(&app, Input::F12, false),
        Some(Action::OpenApplicationMenu)
    );
    assert_eq!(
        direct_menu_shortcut_action(&app, Input::Char('m'), false),
        None
    );
}
