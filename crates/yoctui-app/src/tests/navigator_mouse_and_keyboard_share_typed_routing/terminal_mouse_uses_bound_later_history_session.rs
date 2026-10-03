use super::*;

#[test]
fn terminal_mouse_uses_bound_later_history_session_not_pane_ordinal() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = Screen::TerminalSessions;
    app.focus = FocusTarget::Workspace;
    for id in 1..=27 {
        app.daemon
            .pty_sessions
            .push(yoctui_model::ClientDaemonPtySummary {
                id,
                name: format!("shell-{id}"),
                lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
                viewers: 1,
            });
    }
    app.pty_selection = 25;
    let first = app.pane_layout.focused;
    let second = app.split_terminal_pane(SplitAxis::Horizontal).unwrap();
    yoctui_model::update(&mut app, Action::SelectPtySession { delta: 1 });
    assert_eq!(app.selected_terminal_session().unwrap().id, 27);
    let [nav, workspace, _] = workbench_pane_widths(&app, 160, 50);
    let click = |app: &yoctui_model::App, column| {
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column,
                row: 11,
            },
            app,
            160,
            50,
        )
    };
    let left = click(&app, nav + 3).unwrap();
    assert_eq!(
        left,
        Action::SelectPtyPane {
            pane: first,
            index: 25
        }
    );
    yoctui_model::update(&mut app, left);
    assert_eq!(app.selected_terminal_session().unwrap().id, 26);
    let right = click(&app, nav + workspace - 3).unwrap();
    assert_eq!(
        right,
        Action::SelectPtyPane {
            pane: second,
            index: 26
        }
    );
    yoctui_model::update(&mut app, right);
    assert_eq!(app.selected_terminal_session().unwrap().id, 27);
    assert_eq!(app.terminal_pane_session_index(first, 0, 2), Some(25));
    assert_eq!(app.daemon.pty_sessions.len(), 27);
}
