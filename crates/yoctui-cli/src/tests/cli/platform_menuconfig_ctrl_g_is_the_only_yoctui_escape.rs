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

#[test]
fn menuconfig_encodes_application_cursor_and_control_keys() {
    for (key, normal, application) in [
        (KeyCode::Up, "\x1b[A", "\x1bOA"),
        (KeyCode::Down, "\x1b[B", "\x1bOB"),
        (KeyCode::Right, "\x1b[C", "\x1bOC"),
        (KeyCode::Left, "\x1b[D", "\x1bOD"),
        (KeyCode::Home, "\x1b[H", "\x1bOH"),
        (KeyCode::End, "\x1b[F", "\x1bOF"),
    ] {
        let event = KeyEvent::new(key, KeyModifiers::NONE);
        assert_eq!(terminal_key_bytes(event, false).unwrap(), normal.as_bytes());
        assert_eq!(
            terminal_key_bytes(event, true).unwrap(),
            application.as_bytes()
        );
    }
    for (key, mods, expected) in [
        (KeyCode::Enter, KeyModifiers::NONE, "\r"),
        (KeyCode::Esc, KeyModifiers::NONE, "\x1b"),
        (KeyCode::Tab, KeyModifiers::NONE, "\t"),
        (KeyCode::Char('y'), KeyModifiers::NONE, "y"),
        (KeyCode::Char('n'), KeyModifiers::NONE, "n"),
        (KeyCode::Char('/'), KeyModifiers::NONE, "/"),
        (KeyCode::Char('d'), KeyModifiers::CONTROL, "\x04"),
        (KeyCode::Char('x'), KeyModifiers::ALT, "\x1bx"),
        (KeyCode::Down, KeyModifiers::CONTROL, "\x1b[1;5B"),
        (KeyCode::F(11), KeyModifiers::NONE, "\x1b[23~"),
    ] {
        assert_eq!(
            terminal_key_bytes(KeyEvent::new(key, mods), true).unwrap(),
            expected.as_bytes()
        );
    }
    assert_eq!(
        terminal_key_bytes(
            KeyEvent::new_with_kind(
                KeyCode::Down,
                KeyModifiers::NONE,
                crossterm::event::KeyEventKind::Release
            ),
            true
        ),
        None
    );
}

#[test]
fn menuconfig_key_route_resumes_both_platforms_before_screen_shortcuts() {
    for (screen, name) in [
        (Screen::Kernel, "kernel menuconfig"),
        (Screen::Firmware, "u-boot menuconfig"),
    ] {
        let mut app = App::new(10, 1_000);
        app.workspace.build_dir = Some("/work/build".into());
        app.screen = screen;
        app.daemon.status = ClientReplicaStatus::Current;
        app.terminal.client_id = Some([3; 16]);
        app.daemon.pty_sessions.push(ClientDaemonPtySummary {
            id: 7,
            name: name.into(),
            lifecycle: ClientDaemonLifecycle::Running,
            viewers: 0,
        });
        app.daemon.pty_details.push(ClientDaemonPtyDetails {
            id: 7,
            kind: ClientDaemonPtyKind::Menuconfig,
            cwd: "/work/build".into(),
            columns: 120,
            rows: 40,
            writer: Some([3; 16]),
            writer_epoch: 3,
            exit_code: None,
            restartable: true,
        });
        app.reconcile_platform_menuconfigs();
        assert_eq!(begin_startup_platform_inspection(&mut app), None);
        let toggle = KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL);
        assert_eq!(platform_menuconfig_key(&mut app, toggle), Some(None));
        assert!(app.platform_menuconfig_visible());
        app.focus = FocusTarget::Navigator;
        app.terminal.mode = yoctui_model::TerminalWorkbenchMode::Copy;
        for key in [
            KeyCode::Enter,
            KeyCode::Char('y'),
            KeyCode::Char('/'),
            KeyCode::Up,
            KeyCode::F(12),
        ] {
            assert!(matches!(
                platform_menuconfig_key(&mut app, KeyEvent::new(key, KeyModifiers::NONE)),
                Some(Some(Effect::Terminal(
                    yoctui_model::TerminalEffect::Input { session_id: 7, .. }
                )))
            ));
        }
        assert_eq!(platform_menuconfig_key(&mut app, toggle), Some(None));
        assert!(app.platform_menuconfig_hidden());
        app.screen = Screen::Dashboard;
        assert_eq!(platform_menuconfig_key(&mut app, toggle), None);
        app.screen = screen;
        assert_eq!(platform_menuconfig_key(&mut app, toggle), Some(None));
        assert!(app.platform_menuconfig_visible());
        assert_eq!(app.focus, FocusTarget::Workspace);
        assert_eq!(app.terminal.mode, yoctui_model::TerminalWorkbenchMode::Live);
    }
}
