use super::*;

fn marker_app() -> App {
    let mut app = super::demo_terminal_pane_binding::terminal_binding_fixture();
    let screen = app
        .daemon
        .pty_screens
        .iter_mut()
        .find(|screen| screen.session_id == 26)
        .unwrap();
    screen.columns = 512;
    screen.rows_count = 512;
    screen.rows = vec!["¤".repeat(512); 512];
    app
}

fn assert_visible_cells(app: &App, width: u16, height: u16) {
    let expected = yoctui_app::terminal_workspace_dimensions(app, width, height).unwrap();
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal
        .draw(|frame| render_at(frame, app, literal_now()))
        .unwrap();
    let buffer = terminal.backend().buffer();
    let filled_rows = (0..height)
        .map(|row| {
            (0..width)
                .filter(|col| buffer[(*col, row)].symbol() == "¤")
                .count()
        })
        .filter(|count| *count != 0)
        .collect::<Vec<_>>();
    assert_eq!(
        filled_rows.len(),
        usize::from(expected.rows),
        "{width}x{height}"
    );
    assert!(
        filled_rows
            .iter()
            .all(|count| *count == usize::from(expected.columns)),
        "{width}x{height}: {filled_rows:?}"
    );
}

#[test]
fn terminal_visible_dimensions_match_actual_rendered_cells_across_sizes() {
    let mut app = marker_app();
    for (width, height) in [(160, 50), (150, 50), (200, 60), (120, 30), (80, 24)] {
        assert_visible_cells(&app, width, height);
    }
    app.zoomed_pane = Some(FocusTarget::Workspace);
    assert_visible_cells(&app, 160, 50);
}

#[test]
fn terminal_visible_dimensions_match_search_history_and_split_content() {
    let mut app = marker_app();
    app.terminal.query = "needle".into();
    app.daemon
        .pty_screens
        .last_mut()
        .unwrap()
        .dropped_line_feeds_lower_bound = 2;
    assert_visible_cells(&app, 160, 50);
    let first = app.pane_layout.focused;
    app.split_terminal_pane(yoctui_model::SplitAxis::Horizontal)
        .unwrap();
    app.select_terminal_session(-1);
    assert!(app.select_terminal_pane(first, 2));
    assert_visible_cells(&app, 160, 50);
    app.split_terminal_pane(yoctui_model::SplitAxis::Vertical)
        .unwrap();
    // Only the focused replica is marked; its sibling is explicitly unbound.
    let focused = app.pane_layout.focused;
    app.terminal
        .pane_sessions
        .iter_mut()
        .for_each(|(pane, id)| {
            if *pane != focused {
                *id = None;
            }
        });
    assert_visible_cells(&app, 160, 50);
}

#[test]
fn terminal_visible_dimensions_preserve_menuconfig_and_inplace_geometry() {
    let mut app = marker_app();
    app.daemon.pty_details.last_mut().unwrap().kind = yoctui_model::ClientDaemonPtyKind::Menuconfig;
    assert_visible_cells(&app, 160, 50);
    app.screen = Screen::Kernel;
    app.kernel.menuconfig_terminal = yoctui_model::PlatformTerminalState {
        name: Some("kernel menuconfig".into()),
        session_id: Some(26),
        foreground: true,
        ..yoctui_model::PlatformTerminalState::default()
    };
    app.daemon
        .pty_screens
        .last_mut()
        .unwrap()
        .application_cursor = true;
    assert_visible_cells(&app, 160, 50);
}
