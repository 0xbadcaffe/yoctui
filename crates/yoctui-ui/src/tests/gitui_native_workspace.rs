use super::*;
use yoctui_model::{
    ClientDaemonLifecycle, ClientDaemonPtyDetails, ClientDaemonPtyKind, ClientDaemonPtyScreen,
    ClientDaemonPtySummary, ClientReplicaStatus,
};

#[test]
fn gitui_native_workspace_hides_passive_rails_and_matches_actual_rendered_cells() {
    let mut app = App::new(16, 4096);
    app.onboarding.open = false;
    app.screen = Screen::TerminalSessions;
    app.focus = FocusTarget::Workspace;
    app.daemon.status = ClientReplicaStatus::Current;
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
        writer: None,
        writer_epoch: 7,
        exit_code: None,
        restartable: true,
    });
    for (w, h) in [(160, 50), (120, 32), (100, 30), (80, 24)] {
        let dimensions = yoctui_app::terminal_workspace_dimensions(&app, w, h).unwrap();
        app.daemon.pty_screens = vec![ClientDaemonPtyScreen {
            session_id: 41,
            columns: dimensions.columns,
            rows_count: dimensions.rows,
            cursor_column: 0,
            cursor_row: 0,
            cursor_hidden: true,
            application_cursor: false,
            scrollback_offset: 0,
            rows: (0..dimensions.rows)
                .map(|n| {
                    if n + 1 == dimensions.rows {
                        format!("{}END", "x".repeat(usize::from(dimensions.columns - 3)))
                    } else {
                        "GitUI native row".into()
                    }
                })
                .collect(),
            cells: Vec::new(),
            scrollback_lines: 0,
            dropped_line_feeds_lower_bound: 0,
        }];
        let text = rendered_text(&app, w, h);
        assert!(
            text.contains("END"),
            "{w}x{h}: native last column/row must remain visible: {text}"
        );
        assert!(!text.contains("Inspector:"), "{text}");
        assert!(!text.contains("Prefix help"), "{text}");
        assert!(
            text.contains("viewer"),
            "writer ownership must stay explicit: {text}"
        );
    }
    app.daemon.pty_sessions[0].name = "ordinary utility".into();
    let text = rendered_text(&app, 160, 50);
    assert!(text.contains("Inspector:"));
    assert!(text.contains("Prefix help"));
    for (w, h) in [(20, 5), (1, 1)] {
        let _ = rendered_text(&app, w, h);
    }
}
