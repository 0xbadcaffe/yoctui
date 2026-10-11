use super::*;
use yoctui_model::{
    ClientDaemonLifecycle, ClientDaemonPtyDetails, ClientDaemonPtyKind, ClientDaemonPtyScreen,
    ClientDaemonPtySummary, ClientReplicaStatus,
};

#[test]
fn navigator_gitui_mouse_selects_and_launches_existing_embedded_view_at_all_sizes() {
    use yoctui_app::{MouseInput, MouseKind, mouse_action_for_app};
    use yoctui_model::{
        Dialog, SourceGitStatus, SourceGitSummary, TerminalCreationKind, WorkspaceDestination,
    };
    for (screen, width, height) in [
        (Screen::Dashboard, 160, 50),
        (Screen::Dashboard, 100, 30),
        (Screen::Dashboard, 80, 24),
        (Screen::Tasks, 160, 48),
        (Screen::Tasks, 120, 40),
    ] {
        let mut app = App::new(16, 4096);
        app.onboarding.open = false;
        app.screen = screen;
        app.focus = FocusTarget::Navigator;
        app.preferences.mouse_enabled = true;
        app.gitui_program = Some("/usr/bin/gitui".into());
        app.workspace.source_dir = Some("/source".into());
        app.source_git_status = SourceGitStatus::Ready(SourceGitSummary::default());
        app.navigator_selection =
            App::navigator_selection_for_destination(WorkspaceDestination::Settings).unwrap();
        let gitui = App::navigator_selection_for_destination(WorkspaceDestination::GitUi).unwrap();
        for expected in [
            Action::SelectNavigatorAt { index: gitui },
            Action::ActivateNavigator,
        ] {
            let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
            terminal
                .draw(|frame| render_at(frame, &app, literal_now()))
                .unwrap();
            let buffer = terminal.backend().buffer();
            let row = (0..height)
                .find(|row| {
                    (0..26.min(width))
                        .map(|col| buffer[(col, *row)].symbol())
                        .collect::<String>()
                        .contains("GitUI")
                })
                .unwrap_or_else(|| {
                    panic!("missing navigator GitUI at {screen:?} {width}x{height}")
                });
            let action = mouse_action_for_app(
                MouseInput {
                    kind: MouseKind::Down,
                    column: 10,
                    row,
                },
                &app,
                width,
                height,
            )
            .unwrap();
            assert_eq!(action, expected, "{screen:?} {width}x{height}");
            update(&mut app, action);
        }
        let Some(Dialog::TerminalLaunch(dialog)) = app.active_dialog() else {
            panic!("GitUI navigator must open the existing terminal chooser");
        };
        assert_eq!(dialog.request.kind, TerminalCreationKind::GitUi);
        assert_eq!(dialog.request.cwd, PathBuf::from("/source"));
        assert!(
            app.daemon.pty_sessions.is_empty(),
            "selection alone must not create a process"
        );
    }
}

#[test]
fn gitui_native_workspace_hides_passive_rails_and_matches_actual_rendered_cells() {
    let mut app = App::new(16, 4096);
    app.inspector_visible = true;
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
