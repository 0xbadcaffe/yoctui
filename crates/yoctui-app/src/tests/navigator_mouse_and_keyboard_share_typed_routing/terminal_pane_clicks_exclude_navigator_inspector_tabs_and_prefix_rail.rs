use super::*;

#[test]
fn terminal_pane_clicks_leave_navigator_and_exclude_header_and_footer() {
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
    for (width, height) in [(160, 50), (200, 60)] {
        let [nav, _, _] = workbench_pane_widths(&app, width, height);
        let click = |column, row| {
            mouse_action_for_app(
                MouseInput {
                    kind: MouseKind::Down,
                    column,
                    row,
                },
                &app,
                width,
                height,
            )
        };
        assert!(!matches!(click(5, 10), Some(Action::SelectPtyPane { .. })));
        assert!(matches!(
            click(width - 3, 10),
            Some(Action::SelectPtyPane { .. })
        ));
        assert!(!matches!(
            click(nav + 2, 2),
            Some(Action::SelectPtyPane { .. })
        ));
        assert!(!matches!(
            click(nav + 2, height - 1),
            Some(Action::SelectPtyPane { .. })
        ));
        assert!(matches!(
            click(nav + 2, 10),
            Some(Action::SelectPtyPane { index: 0, .. })
        ));
    }
}
