use super::*;

pub(super) fn terminal_binding_fixture() -> App {
    let mut app = App::new(8, 1_000);
    app.screen = Screen::TerminalSessions;
    app.focus = FocusTarget::Workspace;
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.terminal.client_id = Some([7; 16]);
    for id in [1, 2, 26] {
        app.daemon
            .pty_sessions
            .push(yoctui_model::ClientDaemonPtySummary {
                id,
                name: format!("shell-{id}"),
                lifecycle: if id == 26 {
                    yoctui_model::ClientDaemonLifecycle::Running
                } else {
                    yoctui_model::ClientDaemonLifecycle::Exited
                },
                viewers: 1,
            });
        app.daemon
            .pty_details
            .push(yoctui_model::ClientDaemonPtyDetails {
                id,
                kind: yoctui_model::ClientDaemonPtyKind::BuildShell,
                cwd: "/build".into(),
                columns: 80,
                rows: 24,
                writer: (id == 26).then_some([7; 16]),
                writer_epoch: 1,
                exit_code: (id != 26).then_some(0),
                restartable: true,
            });
        app.daemon
            .pty_screens
            .push(yoctui_model::ClientDaemonPtyScreen {
                session_id: id,
                columns: 80,
                rows_count: 24,
                cursor_column: 0,
                cursor_row: 0,
                cursor_hidden: true,
                application_cursor: false,
                scrollback_offset: 0,
                rows: vec![format!("VISIBLE_SESSION_{id}")],
                cells: vec![],
                scrollback_lines: 0,
                dropped_line_feeds_lower_bound: 0,
            });
    }
    app.pty_selection = 2;
    app
}

fn terminal_output(app: &App) -> String {
    rendered_region_rows(200, 30, |frame, area| {
        terminal_workspace::terminal_sessions_workspace(frame, app, area);
    })
    .join("\n")
}

#[test]
fn terminal_split_displays_selected_later_history_session_in_focused_pane() {
    let mut app = terminal_binding_fixture();
    app.pane_layout
        .split(app.pane_layout.focused, yoctui_model::SplitAxis::Horizontal)
        .unwrap();
    assert_eq!(app.selected_terminal_session().unwrap().id, 26);
    assert!(app.selected_terminal_is_writer());
    let output = terminal_output(&app);
    assert!(
        output.contains("VISIBLE_SESSION_26"),
        "focused selected writer output disappeared: {output}"
    );
}

#[test]
fn terminal_bound_output_survives_focus_and_close_without_deleting_history() {
    let mut app = terminal_binding_fixture();
    let first = app.pane_layout.focused;
    let second = app
        .split_terminal_pane(yoctui_model::SplitAxis::Horizontal)
        .unwrap();
    app.select_terminal_session(-1);
    let output = terminal_output(&app);
    assert!(output.contains("VISIBLE_SESSION_26"));
    assert!(output.contains("VISIBLE_SESSION_2"));
    assert!(app.select_terminal_pane(first, 2));
    assert_eq!(app.selected_terminal_session().unwrap().id, 26);
    let focused_output = terminal_output(&app);
    assert!(focused_output.contains("VISIBLE_SESSION_26"));
    assert!(focused_output.contains("VISIBLE_SESSION_2"));
    app.close_terminal_pane().unwrap();
    assert_eq!(app.pane_layout.focused, second);
    assert_eq!(app.selected_terminal_session().unwrap().id, 2);
    assert!(terminal_output(&app).contains("VISIBLE_SESSION_2"));
    assert!(!terminal_output(&app).contains("VISIBLE_SESSION_26"));
    assert_eq!(app.daemon.pty_sessions.len(), 3);
}

#[test]
fn terminal_bound_output_disappears_when_session_is_removed_not_reassigned() {
    let mut app = terminal_binding_fixture();
    app.split_terminal_pane(yoctui_model::SplitAxis::Horizontal)
        .unwrap();
    app.select_terminal_session(-1);
    app.daemon.pty_sessions.retain(|session| session.id != 26);
    app.reconcile_terminal_panes();
    let output = terminal_output(&app);
    assert!(!output.contains("VISIBLE_SESSION_26"));
    assert!(!output.contains("VISIBLE_SESSION_1"));
    assert!(output.contains("VISIBLE_SESSION_2"));
}

#[test]
fn terminal_bound_output_does_not_reuse_ids_from_a_replacement_daemon() {
    let mut app = terminal_binding_fixture();
    app.daemon.instance_id = Some(yoctui_model::DaemonModelInstanceId([1; 16]));
    app.split_terminal_pane(yoctui_model::SplitAxis::Horizontal)
        .unwrap();
    app.daemon.instance_id = Some(yoctui_model::DaemonModelInstanceId([2; 16]));
    app.reconcile_terminal_panes();
    let output = terminal_output(&app);
    assert!(!output.contains("VISIBLE_SESSION_26"));
    assert!(!output.contains("VISIBLE_SESSION_1"));
    assert!(!output.contains("VISIBLE_SESSION_2"));
    assert!(!app.selected_terminal_is_writer());
}
