use super::*;
use yoctui_model::{
    ClientDaemonLifecycle, ClientDaemonPtyDetails, ClientDaemonPtyKind, ClientDaemonPtySummary,
    ClientReplicaStatus, FocusTarget, TerminalWorkbenchMode,
};

#[test]
fn gitui_terminal_viewport_native_keys_preserve_writer_and_modal_permissions() {
    let mut app = App::new(8, 1024);
    app.onboarding.open = false;
    app.screen = Screen::TerminalSessions;
    app.focus = FocusTarget::Workspace;
    app.daemon.status = ClientReplicaStatus::Current;
    app.terminal.client_id = Some([1; 16]);
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
        writer: Some([1; 16]),
        writer_epoch: 7,
        exit_code: None,
        restartable: true,
    });
    assert_eq!(
        yoctui_app::terminal_workspace_dimensions(&app, 160, 50),
        Some(yoctui_model::PtyDimensions {
            columns: 131,
            rows: 35
        })
    );
    for (input, bytes) in [
        (Input::F1, b"\x1bOP".as_slice()),
        (Input::F12, b"\x1b[24~".as_slice()),
        (Input::Tab, b"\t".as_slice()),
        (Input::Char('q'), b"q".as_slice()),
        (Input::CtrlV, b"\x16".as_slice()),
    ] {
        assert!(direct_menu_shortcut_action(&app, input, false).is_none());
        assert!(yoctui_app::terminal_workspace_action(&app, input).is_none());
        assert_eq!(terminal_input_bytes_for_app(&app, input).unwrap(), bytes);
    }
    for (key, expected) in [
        (
            KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL),
            b"\x01".as_slice(),
        ),
        (
            KeyEvent::new(KeyCode::Char('d'), KeyModifiers::CONTROL),
            b"\x04".as_slice(),
        ),
        (
            KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL),
            b"\x1a".as_slice(),
        ),
        (
            KeyEvent::new(KeyCode::Char('v'), KeyModifiers::CONTROL),
            b"\x16".as_slice(),
        ),
        (
            KeyEvent::new(KeyCode::Left, KeyModifiers::ALT | KeyModifiers::CONTROL),
            b"\x1b[1;7D".as_slice(),
        ),
        (
            KeyEvent::new(KeyCode::F(11), KeyModifiers::NONE),
            b"\x1b[23~".as_slice(),
        ),
    ] {
        let Some(Effect::Terminal(yoctui_model::TerminalEffect::Input {
            session_id,
            writer_epoch,
            bytes,
        })) = crate::interactive_runtime::terminal_writer_key_effect(&app, key)
        else {
            panic!("writer must receive {key:?}");
        };
        assert_eq!(session_id, 41);
        assert_eq!(writer_epoch, 7);
        assert_eq!(bytes, expected);
    }
    let key = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
    app.focus = FocusTarget::Navigator;
    assert!(crate::interactive_runtime::terminal_writer_key_effect(&app, key).is_none());
    app.focus = FocusTarget::Workspace;
    app.dialogs.push_back(yoctui_model::Dialog::BuildOptions);
    assert!(crate::interactive_runtime::terminal_writer_key_effect(&app, key).is_none());
    app.dialogs.clear();
    app.terminal.mode = TerminalWorkbenchMode::Search;
    assert!(crate::interactive_runtime::terminal_writer_key_effect(&app, key).is_none());
    app.terminal.mode = TerminalWorkbenchMode::Live;
    app.daemon.pty_details[0].writer = None;
    assert!(crate::interactive_runtime::terminal_writer_key_effect(&app, key).is_none());
    assert!(!yoctui_app::terminal_owns_input(&app));
    assert!(direct_menu_shortcut_action(&app, Input::F12, false).is_some());
    app.daemon.pty_details[0].writer = Some([1; 16]);
    app.terminal.mode = TerminalWorkbenchMode::KillConfirmation;
    assert_eq!(
        crate::interactive_runtime::terminal_kill_review_key(
            &app,
            KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE)
        ),
        Some(None)
    );
}
