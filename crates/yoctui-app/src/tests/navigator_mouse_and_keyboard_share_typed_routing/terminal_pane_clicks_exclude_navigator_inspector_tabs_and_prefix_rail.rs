use super::*;

#[test]
fn terminal_pane_clicks_exclude_navigator_inspector_tabs_and_prefix_rail() {
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
        let [nav, workspace, _] = workbench_pane_widths(&app, width, height);
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
        assert_eq!(click(nav + workspace, 10), None);
        assert_eq!(click(nav + 2, 6), None);
        assert_eq!(click(nav + 2, height - 5), None);
        assert!(matches!(
            click(nav + 2, 10),
            Some(Action::SelectPtyPane { index: 0, .. })
        ));
    }
}
