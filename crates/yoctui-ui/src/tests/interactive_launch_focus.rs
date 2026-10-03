use super::*;

#[test]
fn interactive_launch_focus_renders_new_session_not_old_bound_writer() {
    let mut app = super::demo_terminal_pane_binding::terminal_binding_fixture();
    app.split_terminal_pane(yoctui_model::SplitAxis::Horizontal)
        .unwrap();
    app.screen = Screen::Recipes;
    app.focus = FocusTarget::Navigator;
    app.focus_return = Some(FocusTarget::Navigator);
    app.dialogs
        .push_back(Dialog::TerminalLaunch(yoctui_model::TerminalLaunchDialog {
            request: yoctui_model::TerminalLaunchRequest {
                name: "new-demo-shell".into(),
                kind: yoctui_model::TerminalCreationKind::BuildShell,
                cwd: "/build".into(),
                program: "/bin/sh".into(),
                arguments: vec![],
                completion: None,
            },
            destination: yoctui_model::TerminalLaunchDestination::Embedded,
            output_must_not_exist: None,
        }));
    app.focus = FocusTarget::Dialog;
    assert!(matches!(
        yoctui_model::update(&mut app, Action::ConfirmTerminalLaunch),
        Some(yoctui_model::Effect::Terminal(_))
    ));
    assert_eq!(app.screen, Screen::TerminalSessions);
    assert_eq!(app.focus, FocusTarget::Workspace);
    assert_eq!(app.selected_terminal_session(), None);
    let mut summary = app.daemon.pty_sessions[2].clone();
    summary.id = 27;
    summary.name = "new-demo-shell".into();
    app.daemon.pty_sessions.push(summary);
    let mut details = app.daemon.pty_details[2].clone();
    details.id = 27;
    details.writer = None;
    app.daemon.pty_details.push(details);
    let mut screen = app.daemon.pty_screens[2].clone();
    screen.session_id = 27;
    screen.rows = vec!["NEW_DEMO_SHELL_PROMPT".into()];
    app.daemon.pty_screens.push(screen);
    app.reconcile_terminal_panes();
    assert_eq!(app.selected_terminal_session().unwrap().id, 27);
    assert!(!app.selected_terminal_is_writer());
    for (width, height) in [(160, 50), (100, 30), (80, 24), (40, 12)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| render_at(frame, &app, literal_now()))
            .unwrap();
        if width >= 80 {
            let output = rendered_text_at(&app, width, height, literal_now());
            assert!(output.contains("NEW_DEMO_SHELL_PROMPT"), "{output}");
        }
        assert_eq!(app.daemon.pty_sessions.len(), 4);
    }
}
