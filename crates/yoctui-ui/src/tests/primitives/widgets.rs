    fn widget_styles() -> WidgetStyles {
        WidgetStyles {
            primary: Style::default().fg(Color::White),
            success: Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
            warning: Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
            error: Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            running: Style::default()
                .fg(Color::LightGreen)
                .add_modifier(Modifier::BOLD),
            pending: Style::default().fg(Color::Yellow),
            disabled: Style::default()
                .fg(Color::DarkGray)
                .add_modifier(Modifier::DIM),
            accent: Style::default().fg(Color::Magenta),
            muted: Style::default().fg(Color::Gray),
            informational: Style::default().fg(Color::Cyan),
            progress: Style::default().fg(Color::Green),
            graph_cpu: Style::default().fg(Color::Cyan),
            graph_memory: Style::default().fg(Color::Magenta),
            graph_disk_read: Style::default().fg(Color::Blue),
            graph_disk_write: Style::default().fg(Color::LightBlue),
            graph_network_rx: Style::default().fg(Color::Green),
            graph_network_tx: Style::default().fg(Color::Yellow),
            selected: Style::default().add_modifier(Modifier::REVERSED),
        }
    }

    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    }

    #[test]
    fn ux_widget_primitives_render_semantic_numeric_visual_vocabulary() {
        let styles = widget_styles();
        let options = WidgetRenderOptions::default();
        let gauge = GaugeProjection::determinate("Build", 7, 9, WidgetRole::Progress);
        let meter = GaugeProjection::terminal(
            "Parse",
            4,
            4,
            yoctui_model::WidgetTerminalState::Success,
            "complete",
        );
        let history = HistoryProjection::bounded(
            "CPU",
            WidgetState::Partial,
            WidgetRole::Cpu,
            Some(71),
            [10, 20, 31, 40, 55, 71],
            60,
            "sample gap",
        );
        let bars = BarProjection::bounded(
            WidgetState::Available,
            [
                yoctui_model::BarValue {
                    label: "Packages".into(),
                    value: 32,
                    role: WidgetRole::Accent,
                },
                yoctui_model::BarValue {
                    label: "Files".into(),
                    value: 19,
                    role: WidgetRole::Informational,
                },
            ],
            8,
            "",
        );
        let tabs =
            TabProjection::bounded(["Summary".into(), "History".into(), "Details".into()], 1, 8);
        let legend = LegendProjection::bounded(
            WidgetState::Available,
            [
                yoctui_model::LegendItem {
                    label: "Runtime".into(),
                    value: "32 MiB".into(),
                    role: WidgetRole::Success,
                },
                yoctui_model::LegendItem {
                    label: "Debug".into(),
                    value: "19 MiB".into(),
                    role: WidgetRole::Warning,
                },
            ],
            8,
            "",
        );
        let scroll = ScrollbarProjection::new(11, 8, 4, 12);

        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal
            .draw(|frame| {
                let rows = Layout::vertical([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(4),
                    Constraint::Length(6),
                    Constraint::Length(1),
                    Constraint::Length(4),
                    Constraint::Min(4),
                ])
                .split(frame.area());
                render_semantic_gauge(frame, rows[0], &gauge, styles, options);
                render_semantic_meter(frame, rows[1], &meter, styles, options);
                render_history_chart(frame, rows[2], &history, styles, options);
                render_bar_chart(frame, rows[3], &bars, styles, options);
                render_tabs(frame, rows[4], &tabs, styles, options);
                render_legend(frame, rows[5], &legend, styles, options);
                render_scrollbar(frame, rows[6], scroll, styles, options);
            })
            .unwrap();
        let text = buffer_text(&terminal);
        assert!(text.contains("7/9 (77%)"), "{text}");
        assert!(text.contains("4/4 (100%)"), "{text}");
        assert!(text.contains("CPU 71 · sample gap"), "{text}");
        assert!(text.contains("[History]"), "{text}");
        assert!(text.contains("Runtime: 32 MiB"), "{text}");
        assert!(text.contains("9-12/12"), "{text}");
        assert!(!text.contains('�'), "{text}");
    }

    #[test]
    fn ux_widget_primitives_ascii_no_color_and_reduced_motion_keep_text() {
        let attribute = Style::default().add_modifier(Modifier::BOLD);
        let styles = WidgetStyles {
            primary: Style::default(),
            success: attribute,
            warning: attribute,
            error: attribute.add_modifier(Modifier::UNDERLINED),
            running: attribute,
            pending: Style::default(),
            disabled: Style::default().add_modifier(Modifier::DIM),
            accent: attribute,
            muted: Style::default(),
            informational: attribute,
            progress: attribute,
            graph_cpu: attribute,
            graph_memory: attribute,
            graph_disk_read: attribute,
            graph_disk_write: attribute,
            graph_network_rx: attribute,
            graph_network_tx: attribute,
            selected: Style::default().add_modifier(Modifier::REVERSED),
        };
        let options = WidgetRenderOptions {
            unicode: false,
            reduced_motion: true,
        };
        let active = GaugeProjection::indeterminate("Runqueue progress unknown", "waiting");
        let unavailable = HistoryProjection::bounded(
            "Network",
            WidgetState::Unavailable,
            WidgetRole::NetworkRx,
            None,
            [],
            60,
            "host source missing",
        );
        let bars = BarProjection::bounded(
            WidgetState::Available,
            [yoctui_model::BarValue {
                label: "Unicode 包".into(),
                value: u64::MAX,
                role: WidgetRole::Accent,
            }],
            4,
            "",
        );
        let tabs = TabProjection::bounded(["One".into(), "Two".into()], usize::MAX, 4);

        let mut terminal = Terminal::new(TestBackend::new(40, 8)).unwrap();
        terminal
            .draw(|frame| {
                let rows = Layout::vertical([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(3),
                    Constraint::Length(1),
                    Constraint::Min(1),
                ])
                .split(frame.area());
                render_semantic_gauge(frame, rows[0], &active, styles, options);
                render_history_chart(frame, rows[1], &unavailable, styles, options);
                render_bar_chart(frame, rows[2], &bars, styles, options);
                render_tabs(frame, rows[3], &tabs, styles, options);
                render_scrollbar(
                    frame,
                    rows[4],
                    ScrollbarProjection::new(0, 0, 4, 0),
                    styles,
                    options,
                );
                render_semantic_meter(frame, Rect::default(), &active, styles, options);
            })
            .unwrap();
        let text = buffer_text(&terminal);
        assert!(
            text.contains("> Runqueue progress unknown · active"),
            "{text}"
        );
        assert!(text.contains("! Network unavailable"), "{text}");
        assert!(text.contains("Unicode 包"), "{text}");
        assert!(text.contains("18446744073709551615"), "{text}");
        assert!(text.contains("[Two]"), "{text}");
        assert!(text.contains("0/0"), "{text}");
        assert!(!text.contains('…'), "{text}");
        assert!(!text.contains('█'), "{text}");
        assert!(!text.contains('�'), "{text}");
        assert!(
            terminal
                .backend()
                .buffer()
                .content
                .iter()
                .all(|cell| cell.fg == Color::Reset && cell.bg == Color::Reset)
        );
    }

    #[test]
    fn ux_widget_primitives_explicit_empty_partial_and_failure_never_panic_narrow() {
        let styles = widget_styles();
        let options = WidgetRenderOptions::default();
        let partial = BarProjection::bounded(WidgetState::Partial, [], 10, "2 records omitted");
        let failed =
            LegendProjection::bounded(WidgetState::TerminalFailure, [], 10, "adapter failed");
        let empty_tabs = TabProjection::bounded([], usize::MAX, usize::MAX);
        let mut terminal = Terminal::new(TestBackend::new(8, 3)).unwrap();
        terminal
            .draw(|frame| {
                let rows = Layout::vertical([
                    Constraint::Length(1),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ])
                .split(frame.area());
                render_bar_chart(frame, rows[0], &partial, styles, options);
                render_legend(frame, rows[1], &failed, styles, options);
                render_tabs(frame, rows[2], &empty_tabs, styles, options);
            })
            .unwrap();
        let text = buffer_text(&terminal);
        assert!(text.contains("partial"), "{text}");
        assert!(text.contains("failed"), "{text}");
        assert!(text.contains("empty"), "{text}");
        assert!(!text.contains('�'), "{text}");
    }

    #[test]
    fn ux_widget_primitives_resolve_every_role_through_the_semantic_theme() {
        let theme = crate::SemanticTheme::for_theme(yoctui_model::Theme::HighContrast, true);
        let styles = theme.widget_styles();
        assert_eq!(styles.progress.fg, Some(theme.progress));
        assert_eq!(styles.graph_cpu.fg, Some(theme.graph_cpu));
        assert_eq!(styles.graph_memory.fg, Some(theme.graph_memory));
        assert_eq!(styles.graph_disk_read.fg, Some(theme.graph_disk_read));
        assert_eq!(styles.graph_disk_write.fg, Some(theme.graph_disk_write));
        assert_eq!(styles.graph_network_rx.fg, Some(theme.graph_network_rx));
        assert_eq!(styles.graph_network_tx.fg, Some(theme.graph_network_tx));

        let no_color =
            crate::SemanticTheme::for_theme(yoctui_model::Theme::DarkPro, false).widget_styles();
        for style in [
            no_color.success,
            no_color.warning,
            no_color.error,
            no_color.running,
            no_color.pending,
            no_color.progress,
            no_color.graph_cpu,
            no_color.graph_memory,
            no_color.graph_disk_read,
            no_color.graph_disk_write,
            no_color.graph_network_rx,
            no_color.graph_network_tx,
        ] {
            assert_eq!(style.fg, Some(Color::Reset));
            assert_ne!(style.add_modifier, Modifier::empty());
        }
    }
