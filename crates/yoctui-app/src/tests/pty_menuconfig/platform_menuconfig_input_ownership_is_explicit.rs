use crate::{Input, terminal_owns_input, terminal_workspace_action};
use yoctui_model::{
    App, ClientDaemonLifecycle, ClientDaemonPtyDetails, ClientDaemonPtyKind,
    ClientDaemonPtySummary, ClientReplicaStatus, FocusTarget, PlatformTerminalState, Screen,
};

#[test]
fn platform_menuconfig_input_ownership_is_explicit() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Kernel;
    app.focus = FocusTarget::Workspace;
    app.daemon.status = ClientReplicaStatus::Current;
    app.terminal.client_id = Some([7; 16]);
    app.kernel.menuconfig_terminal = PlatformTerminalState {
        name: Some("kernel menuconfig".into()),
        session_id: Some(52),
        foreground: true,
        writer_control_requested: true,
        ..PlatformTerminalState::default()
    };
    app.daemon.pty_sessions.push(ClientDaemonPtySummary {
        id: 52,
        name: "kernel menuconfig".into(),
        lifecycle: ClientDaemonLifecycle::Running,
        viewers: 1,
    });
    app.daemon.pty_details.push(ClientDaemonPtyDetails {
        id: 52,
        kind: ClientDaemonPtyKind::Menuconfig,
        cwd: "/work/build".into(),
        columns: 120,
        rows: 40,
        writer: Some([7; 16]),
        writer_epoch: 3,
        exit_code: None,
        restartable: true,
    });
    app.daemon
        .pty_screens
        .push(yoctui_model::ClientDaemonPtyScreen {
            session_id: 52,
            columns: 120,
            rows_count: 40,
            cursor_column: 0,
            cursor_row: 0,
            cursor_hidden: false,
            application_cursor: true,
            scrollback_offset: 0,
            rows: vec!["Linux Kernel Configuration".into()],
            cells: Vec::new(),
            scrollback_lines: 0,
            dropped_line_feeds_lower_bound: 0,
        });

    assert!(terminal_owns_input(&app));
    assert_eq!(terminal_workspace_action(&app, Input::F12), None);
    assert_eq!(terminal_workspace_action(&app, Input::Up), None);

    app.toggle_platform_menuconfig_foreground();
    assert!(!terminal_owns_input(&app));
    assert!(app.platform_menuconfig_hidden());
    assert_eq!(app.kernel.menuconfig_terminal.session_id, Some(52));
}
