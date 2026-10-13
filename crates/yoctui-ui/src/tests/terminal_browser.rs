use super::*;

#[test]
fn terminal_browser_tree_filters_old_sessions_and_console_keeps_navigator() {
    let mut app = super::demo_terminal_pane_binding::terminal_binding_fixture();
    app.terminal.browser_open = true;
    for (width, height) in [(80, 24), (100, 30), (160, 48), (240, 80)] {
        let text = rendered_text(&app, width, height);
        assert!(text.contains("session tree"), "{text}");
        assert!(text.contains("shell-26"), "{text}");
        assert!(
            !text.contains("shell-1"),
            "unavailable sessions must be hidden: {text}"
        );
        assert!(
            !text.contains("VISIBLE_SESSION_26"),
            "list must not embed terminal output: {text}"
        );
    }
    update(&mut app, Action::TerminalTakeControl);
    for (width, height) in [(80, 24), (100, 30), (160, 48), (240, 80)] {
        let text = rendered_text(&app, width, height);
        assert!(text.contains("VISIBLE_SESSION_26"), "{text}");
        assert!(text.contains("Navigator"), "{text}");
        assert!(!text.contains("Inspector:"), "{text}");
        assert!(!text.contains("session tree"), "{text}");
    }
    update(&mut app, Action::TerminalReleaseControl);
    assert!(rendered_text(&app, 160, 48).contains("session tree"));
}

#[test]
fn terminal_browser_end_home_and_mouse_match_current_tree_viewport() {
    use yoctui_app::{MouseInput, MouseKind, mouse_action_for_app};
    let mut app = super::demo_terminal_pane_binding::terminal_binding_fixture();
    app.preferences.mouse_enabled = true;
    app.terminal.browser_open = true;
    for id in 27..=126 {
        let mut session = app.daemon.pty_sessions[2].clone();
        session.id = id;
        session.name = format!("shell-{id}");
        app.daemon.pty_sessions.push(session);
    }
    update(&mut app, Action::SelectPtySession { delta: isize::MAX });
    let text = rendered_text(&app, 160, 48);
    assert!(text.contains("shell-126"), "{text}");
    let rows = rendered_region_rows(160, 48, |frame, _| render_at(frame, &app, literal_now()));
    let row = rows
        .iter()
        .position(|line| line.contains("shell-126"))
        .unwrap() as u16;
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 50,
                row
            },
            &app,
            160,
            48
        ),
        Some(Action::TerminalTakeControl)
    );
    update(&mut app, Action::SelectPtySession { delta: isize::MIN });
    assert_eq!(app.selected_terminal_session().unwrap().id, 26);
    let text = rendered_text(&app, 160, 48);
    assert!(text.contains("shell-26"), "{text}");
    assert!(
        !text.contains("shell-126"),
        "current viewport must return to top: {text}"
    );
}
