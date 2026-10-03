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
fn function_keys_are_forwarded_while_an_embedded_menuconfig_owns_input() {
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
        foreground: true,
        ..yoctui_model::PlatformTerminalState::default()
    };

    assert!(yoctui_app::terminal_owns_input(&app));
    assert_eq!(direct_menu_shortcut_action(&app, Input::F2, false), None);
    assert_eq!(direct_menu_shortcut_action(&app, Input::F12, false), None);
    assert_eq!(
        direct_menu_shortcut_action(&app, Input::Char('m'), false),
        None
    );
}

#[test]
fn function_keys_keep_global_routes_for_read_only_terminal_history() {
    use yoctui_model::{ClientDaemonLifecycle as Life, ClientReplicaStatus as Status};
    let mut app = App::new(8, 1_000);
    app.screen = Screen::TerminalSessions;
    app.focus = yoctui_model::FocusTarget::Workspace;
    app.terminal.client_id = Some([4; 16]);
    app.daemon
        .pty_sessions
        .push(yoctui_model::ClientDaemonPtySummary {
            id: 9,
            name: "shell history".into(),
            lifecycle: Life::Running,
            viewers: 1,
        });
    app.daemon
        .pty_details
        .push(yoctui_model::ClientDaemonPtyDetails {
            id: 9,
            kind: yoctui_model::ClientDaemonPtyKind::BuildShell,
            cwd: "/build".into(),
            columns: 80,
            rows: 24,
            writer: Some([4; 16]),
            writer_epoch: 1,
            exit_code: None,
            restartable: true,
        });
    for status in [Status::Current, Status::Stale, Status::Disconnected] {
        app.daemon.status = status;
        for life in [
            Life::Running,
            Life::Connecting,
            Life::Stopping,
            Life::Exited,
            Life::Failed,
            Life::Lost,
            Life::Disconnected,
        ] {
            app.daemon.pty_sessions[0].lifecycle = life;
            for writer in [None, Some([4; 16]), Some([5; 16])] {
                app.daemon.pty_details[0].writer = writer;
                for key in [
                    Input::F1,
                    Input::F2,
                    Input::F3,
                    Input::F4,
                    Input::F5,
                    Input::F6,
                    Input::F7,
                    Input::F8,
                    Input::F9,
                    Input::F12,
                ] {
                    assert_eq!(
                        direct_menu_shortcut_action(&app, key, false),
                        yoctui_app::key_action(key),
                        "{status:?}/{life:?}/{writer:?}/{key:?}"
                    );
                }
                let action = direct_menu_shortcut_action(&app, Input::F4, false).unwrap();
                assert!(update(&mut app, action).is_none());
                assert_eq!(app.screen, Screen::Dashboard);
                app.screen = Screen::TerminalSessions;
                assert_eq!(
                    direct_menu_shortcut_action(&app, Input::Char('n'), false),
                    None
                );
            }
        }
    }
    app.daemon.pty_sessions.clear();
    app.daemon.pty_details.clear();
    assert_eq!(
        direct_menu_shortcut_action(&app, Input::F4, false),
        Some(Action::Open(Screen::Dashboard))
    );
}

#[test]
fn function_keys_do_not_escape_dialog_palette_or_context_replay() {
    let mut app = App::new(8, 1_000);
    app.screen = Screen::TerminalSessions;
    assert_eq!(direct_menu_shortcut_action(&app, Input::F4, true), None);
    app.command_palette_open = true;
    assert_eq!(direct_menu_shortcut_action(&app, Input::F4, false), None);
    app.command_palette_open = false;
    let _ = update(&mut app, Action::Quit);
    assert!(app.active_dialog().is_some());
    assert_eq!(direct_menu_shortcut_action(&app, Input::F4, false), None);
    assert_eq!(
        direct_menu_shortcut_action(&app, Input::F12, false),
        Some(Action::OpenApplicationMenu)
    );
}
