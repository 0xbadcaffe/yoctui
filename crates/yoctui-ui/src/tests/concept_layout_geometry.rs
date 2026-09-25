use super::*;

#[test]
fn dashboard_concept_has_distinct_regions_and_resizes_without_mutation() {
    let app = concept_idle_dashboard_app();
    for (width, height) in [(150, 50), (160, 50), (180, 55), (200, 60)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| render_at(frame, &app, literal_now()))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let [nav, work, _] = yoctui_app::workbench_pane_widths(&app, width, height);
        for x in [0, nav, nav + work] {
            assert_eq!(buffer[(x, 5)].symbol(), "┌");
        }
        let region_text = |x: u16, y: u16, w: u16, h: u16| {
            (y..y + h)
                .flat_map(|row| (x..x + w).map(move |col| buffer[(col, row)].symbol()))
                .collect::<String>()
        };
        assert!(region_text(nav, 5, work, 9).contains("Build Overview"));
        assert!(region_text(nav, 14, work, height - 35).contains("Recent Builds"));
        let telemetry = region_text(nav, height - 18, work, 8);
        assert!(telemetry.contains("Resource Telemetry"));
        assert!(telemetry.contains("4 cores"));
        assert!(telemetry.contains("6.7/16.0 GiB"));
        assert!(telemetry.contains("55.5 GiB free"));
        assert!(
            !telemetry.contains('⣿'),
            "Dashboard reuses the Tasks segmented bars rather than dotted arcs"
        );
        assert!(region_text(nav, height - 10, work, 7).contains("Quick Actions"));
        assert!(
            region_text(nav + work, 5, width - nav - work, height - 8)
                .contains("Project Inspector")
        );
        assert!(!buffer.content.iter().any(|cell| {
            cell.symbol()
                .chars()
                .any(|ch| ('\u{2801}'..='\u{28ff}').contains(&ch))
        }));
        for percent in ["18%", "42%", "63%"] {
            assert!(telemetry.contains(percent));
        }
        assert_eq!(app.screen, Screen::Dashboard);
    }
    let mut empty = App::new(16, 4096);
    empty.focus = FocusTarget::Workspace;
    let unavailable = rendered_text_at(&empty, 160, 50, literal_now());
    assert!(unavailable.contains("unavailable"));
    assert!(!unavailable.contains("CPU Usage 0%"));
    empty.preferences.symbols = SymbolPreference::Ascii;
    let ascii = rendered_text_at(&empty, 160, 50, literal_now());
    assert!(
        !ascii
            .chars()
            .any(|ch| ('\u{2801}'..='\u{28ff}').contains(&ch))
    );
}

#[test]
fn concept_terminal_tabs_prefix_and_panes_stay_visible_across_resize() {
    let app = concept_terminal_sessions_app();
    for (width, height) in [(150, 50), (160, 50), (200, 60)] {
        let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
        terminal
            .draw(|frame| render_at(frame, &app, literal_now()))
            .unwrap();
        let buffer = terminal.backend().buffer();
        let [nav, workspace, _] = yoctui_app::workbench_pane_widths(&app, width, height);
        let row = |y| {
            (nav..nav + workspace)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        };
        assert!(row(6).contains("1:shell"), "tabs must occupy a content row");
        assert!(row(6).contains("2:devshell"));
        assert_eq!(buffer[(nav, 9)].symbol(), "┌");
        assert!(row(height - 7).contains("Prefix help"));
        assert_eq!(app.pty_selection, 0);
    }
}

#[test]
fn retained_terminal_panes_never_claim_writer_access() {
    let mut app = concept_terminal_sessions_app();
    app.daemon.status = yoctui_model::ClientReplicaStatus::Stale;
    let output = rendered_text_at(&app, 160, 50, literal_now());
    assert!(output.contains("BuildShell · retained read-only"));
    assert!(!output.contains("BuildShell · writer"));
}
