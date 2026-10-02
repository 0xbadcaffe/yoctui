use super::*;

fn header_buffer(app: &App, width: u16) -> ratatui::buffer::Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, 5)).unwrap();
    terminal
        .draw(|frame| workbench_header(frame, app, frame.area(), literal_now()))
        .unwrap();
    terminal.backend().buffer().clone()
}

fn identity_row(buffer: &ratatui::buffer::Buffer) -> String {
    (0..buffer.area.width)
        .map(|x| buffer[(x, 1)].symbol())
        .collect()
}

#[test]
fn header_omits_complete_optional_metadata_before_it_clips_into_health() {
    let mut app = App::new(32, 8192);
    app.workspace.source_dir = Some("/workspace/openbmc".into());
    app.build.target = Some("obmc-phosphor-image".into());
    app.workspace
        .variables
        .insert("MACHINE".into(), "romulus".into());
    app.workspace
        .variables
        .insert("DISTRO".into(), "openbmc-openpower".into());
    app.daemon.status = yoctui_model::ClientReplicaStatus::Current;
    app.daemon.bitbake = yoctui_model::ClientDaemonLifecycle::Running;
    for width in [80, 100, 130, 160, 180, 200, 240] {
        let buffer = header_buffer(&app, width);
        let row = identity_row(&buffer);
        assert!(
            row.contains(concat!("yoctui v", env!("CARGO_PKG_VERSION"))),
            "{width}: {row}"
        );
        assert!(row.contains("Connected/Local"), "{row}");
        assert!(row.contains("19:28"), "{row}");
        if width < 180 {
            assert!(!row.contains("Distro:"), "{row}");
        }
        if width < 130 {
            assert!(!row.contains("Machine:"), "{row}");
        }
        if row.contains("Machine:") {
            assert!(row.contains("Machine: romulus"), "partial metadata: {row}");
        }
        if row.contains("Distro:") {
            assert!(
                row.contains("Distro: openbmc-openpower"),
                "partial metadata: {row}"
            );
        }
        if width == 240 {
            assert!(row.contains("Machine: romulus"), "{row}");
            assert!(row.contains("Distro: openbmc-openpower"), "{row}");
        }
        assert_eq!(app.workspace.variables["DISTRO"], "openbmc-openpower");
    }
}

#[test]
fn long_unicode_primary_context_is_explicitly_bounded_without_losing_version_or_style() {
    for color_enabled in [true, false] {
        let mut app = App::new(32, 8192);
        app.color_enabled = color_enabled;
        app.workspace.source_dir = Some(format!("/workspace/{}", "工程".repeat(50)).into());
        app.build.target = Some("image-with-an-extremely-long-name".repeat(20));
        app.source_git_status =
            yoctui_model::SourceGitStatus::Ready(yoctui_model::SourceGitSummary {
                branch: "feature/工程".repeat(50),
                ..Default::default()
            });
        for width in [80, 100, 130, 160] {
            let buffer = header_buffer(&app, width);
            let row = identity_row(&buffer);
            assert!(
                row.contains(concat!("yoctui v", env!("CARGO_PKG_VERSION"))),
                "{row}"
            );
            assert!(row.contains("Disconnected/Local"), "{row}");
            assert!(row.contains("19:28"), "{row}");
            assert!(row.contains('…'), "unmarked primary clipping: {row}");
            assert!(!row.contains('�'), "{row}");
            let ellipsis = (0..width)
                .find(|x| buffer[(*x, 1)].symbol() == "…")
                .unwrap();
            let palette = ThemePalette::for_app(&app);
            let expected = palette.role(palette.informational, Modifier::BOLD);
            assert_eq!(buffer[(ellipsis, 1)].fg, expected.fg.unwrap());
            assert_eq!(buffer[(ellipsis, 1)].modifier, expected.add_modifier);
        }
    }
}

#[test]
fn header_span_fitting_marks_exact_boundary_and_preserves_wide_cell_budget() {
    let style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    for value in ["abcd", "工程a"] {
        let spans = vec![Span::styled(value, style), Span::raw(" trailing")];
        let fitted = crate::header::fit_header_spans(spans, 4);
        let line = Line::from(fitted);
        assert!(line.width() <= 4);
        assert!(line.to_string().ends_with('…'));
        assert_eq!(line.spans.last().unwrap().style, style);
    }
}
