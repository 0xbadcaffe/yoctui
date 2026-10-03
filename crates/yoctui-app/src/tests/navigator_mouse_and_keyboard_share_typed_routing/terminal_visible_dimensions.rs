use super::*;

fn fixture() -> yoctui_model::App {
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = Screen::TerminalSessions;
    app.focus = FocusTarget::Workspace;
    app.daemon
        .pty_sessions
        .push(yoctui_model::ClientDaemonPtySummary {
            id: 26,
            name: "shell".into(),
            lifecycle: yoctui_model::ClientDaemonLifecycle::Running,
            viewers: 1,
        });
    app.daemon
        .pty_details
        .push(yoctui_model::ClientDaemonPtyDetails {
            id: 26,
            kind: yoctui_model::ClientDaemonPtyKind::BuildShell,
            cwd: "/build".into(),
            columns: 120,
            rows: 40,
            writer: None,
            writer_epoch: 0,
            exit_code: None,
            restartable: true,
        });
    app.daemon
        .pty_screens
        .push(yoctui_model::ClientDaemonPtyScreen {
            session_id: 26,
            columns: 120,
            rows_count: 40,
            cursor_column: 0,
            cursor_row: 0,
            cursor_hidden: true,
            application_cursor: false,
            scrollback_offset: 0,
            rows: vec![],
            cells: vec![],
            scrollback_lines: 0,
            dropped_line_feeds_lower_bound: 0,
        });
    app
}

fn size(columns: u16, rows: u16) -> Option<yoctui_model::PtyDimensions> {
    Some(yoctui_model::PtyDimensions { columns, rows })
}

#[test]
fn terminal_ordinary_dimensions_include_prefix_search_and_history_rows() {
    let mut app = fixture();
    assert_eq!(terminal_workspace_dimensions(&app, 160, 50), size(98, 32));
    app.daemon.pty_screens[0].scrollback_lines = 20;
    assert_eq!(terminal_workspace_dimensions(&app, 160, 50), size(98, 31));
    app.terminal.query = "find".into();
    assert_eq!(terminal_workspace_dimensions(&app, 160, 50), size(98, 30));
    app.daemon.pty_screens.clear();
    assert_eq!(terminal_workspace_dimensions(&app, 160, 50), size(98, 32));
}

#[test]
fn terminal_dimensions_follow_focused_pane_not_global_history_index() {
    let mut app = fixture();
    for id in 1..=25 {
        let mut session = app.daemon.pty_sessions[0].clone();
        session.id = id;
        app.daemon.pty_sessions.insert(0, session);
    }
    app.pty_selection = 25;
    let first = app.pane_layout.focused;
    app.split_terminal_pane(yoctui_model::SplitAxis::Horizontal)
        .unwrap();
    app.pane_layout.resize(first, 200).unwrap();
    assert_eq!(terminal_workspace_dimensions(&app, 160, 50), size(28, 32));
    assert!(app.select_terminal_pane(first, 25));
    assert_eq!(terminal_workspace_dimensions(&app, 160, 50), size(68, 32));
    app.split_terminal_pane(yoctui_model::SplitAxis::Vertical)
        .unwrap();
    assert_eq!(terminal_workspace_dimensions(&app, 160, 50), size(68, 15));
}

#[test]
fn terminal_dimensions_exclude_hidden_small_and_nonlive_workspaces() {
    let mut app = fixture();
    assert_eq!(terminal_workspace_dimensions(&app, 80, 24), size(78, 8));
    app.focus = FocusTarget::Navigator;
    assert_eq!(terminal_workspace_dimensions(&app, 80, 24), None);
    app.zoomed_pane = Some(FocusTarget::Workspace);
    assert_eq!(terminal_workspace_dimensions(&app, 160, 50), size(158, 31));
    app.zoomed_pane = Some(FocusTarget::Inspector);
    assert_eq!(terminal_workspace_dimensions(&app, 160, 50), None);
    app.zoomed_pane = None;
    app.focus = FocusTarget::Workspace;
    for (width, height) in [(0, 0), (79, 24), (160, 23)] {
        assert_eq!(terminal_workspace_dimensions(&app, width, height), None);
    }
    for mode in [
        yoctui_model::TerminalWorkbenchMode::Copy,
        yoctui_model::TerminalWorkbenchMode::Search,
        yoctui_model::TerminalWorkbenchMode::Help,
    ] {
        app.terminal.mode = mode;
        assert_eq!(terminal_workspace_dimensions(&app, 160, 50), None);
    }
}

#[test]
fn terminal_dimensions_never_inflate_an_empty_split_into_writable_cells() {
    let mut app = fixture();
    for _ in 0..5 {
        app.split_terminal_pane(yoctui_model::SplitAxis::Vertical)
            .unwrap();
    }
    assert_eq!(terminal_workspace_dimensions(&app, 160, 50), None);
    app.daemon.pty_sessions.clear();
    assert_eq!(terminal_workspace_dimensions(&app, 160, 50), None);
}
