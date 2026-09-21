use super::*;

#[test]
fn mouse_runtime_routes_dialog_and_terminal_session_clicks() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = Screen::TerminalSessions;
    app.daemon
        .pty_sessions
        .push(yoctui_model::ClientDaemonPtySummary {
            id: 1,
            name: "shell".into(),
            lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
            viewers: 1,
        });
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 40,
                row: 7
            },
            &app,
            120,
            30,
        ),
        Some(Action::SelectPtyPane {
            pane: yoctui_model::PaneId(1),
            index: 0,
        })
    );
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Drag,
                column: 40,
                row: 7,
            },
            &app,
            120,
            30,
        ),
        None,
        "a single terminal leaf has no supported split to resize"
    );
    app.dialogs.push_back(yoctui_model::Dialog::BuildOptions);
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 40,
                row: 7
            },
            &app,
            120,
            30,
        ),
        Some(Action::Focus(FocusTarget::Dialog))
    );
}
