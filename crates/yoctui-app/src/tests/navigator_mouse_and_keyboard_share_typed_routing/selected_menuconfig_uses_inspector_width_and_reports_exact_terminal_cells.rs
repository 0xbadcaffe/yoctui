use super::*;

#[test]
fn selected_menuconfig_uses_inspector_width_and_reports_exact_terminal_cells() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = Screen::TerminalSessions;
    app.daemon
        .pty_sessions
        .push(yoctui_model::ClientDaemonPtySummary {
            id: 9,
            name: "menuconfig:virtual/kernel".into(),
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
            writer: None,
            writer_epoch: 0,
            exit_code: None,
            restartable: true,
        });

    assert_eq!(workbench_pane_widths(&app, 160, 50), [27, 133, 0]);
    assert_eq!(
        terminal_workspace_dimensions(&app, 160, 50),
        Some(yoctui_model::PtyDimensions {
            columns: 131,
            rows: 35,
        })
    );

    app.daemon.pty_details[0].kind = yoctui_model::ClientDaemonPtyKind::BuildShell;
    assert_eq!(workbench_pane_widths(&app, 160, 50), [27, 100, 33]);
    assert_eq!(terminal_workspace_dimensions(&app, 160, 50), None);
}
