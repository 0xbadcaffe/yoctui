use super::*;
use yoctui_model::{
    ClientDaemonLifecycle, ClientDaemonPtyDetails, ClientDaemonPtyKind, ClientDaemonPtySummary,
    ClientReplicaStatus, FocusTarget, PlatformTerminalState,
};

#[test]
fn platform_menuconfig_ctrl_g_is_the_only_yoctui_escape() {
    let ctrl_g = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL);
    assert_eq!(input_from_key(ctrl_g), Some(Input::CtrlG));
    assert_eq!(terminal_input_bytes(Input::CtrlG), Some(vec![0x07]));

    let mut app = App::new(10, 1_000);
    app.screen = Screen::Kernel;
    app.focus = FocusTarget::Workspace;
    app.daemon.status = ClientReplicaStatus::Current;
    app.terminal.client_id = Some([3; 16]);
    app.kernel.menuconfig_terminal = PlatformTerminalState {
        name: Some("kernel menuconfig".into()),
        session_id: Some(7),
        foreground: true,
        writer_control_requested: true,
        ..PlatformTerminalState::default()
    };
    app.daemon.pty_sessions.push(ClientDaemonPtySummary {
        id: 7,
        name: "kernel menuconfig".into(),
        lifecycle: ClientDaemonLifecycle::Running,
        viewers: 1,
    });
    app.daemon.pty_details.push(ClientDaemonPtyDetails {
        id: 7,
        kind: ClientDaemonPtyKind::Menuconfig,
        cwd: "/work/build".into(),
        columns: 120,
        rows: 40,
        writer: Some([3; 16]),
        writer_epoch: 1,
        exit_code: None,
        restartable: true,
    });

    for input in [Input::F1, Input::F4, Input::F12, Input::CtrlB, Input::Up] {
        assert_eq!(direct_menu_shortcut_action(&app, input, false), None);
    }
}
