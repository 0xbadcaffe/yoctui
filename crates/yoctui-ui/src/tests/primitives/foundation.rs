    #[test]
    fn foundation_ui_primitives_render_focus_header_separator_and_states() {
        let styles = styles();
        let shell = PaneShell::new(
            section_header(
                Span::raw("Tasks"),
                Some(Span::styled("Running", Style::default().fg(Color::Green))),
                styles.muted,
            ),
            true,
            styles,
        );
        let state = StateView {
            kind: StateKind::Unavailable,
            summary: "Unavailable — CPU sample missing.".into(),
            detail: Some("Host did not publish this metric.".into()),
            action: None,
        };
        let mut terminal = Terminal::new(TestBackend::new(50, 8)).unwrap();
        terminal
            .draw(|frame| {
                let area = frame.area();
                let inner = shell.clone().block().inner(area);
                frame.render_widget(shell.clone().block(), area);
                frame.render_widget(
                    state.paragraph(
                        Style::default().fg(Color::Yellow),
                        Style::default().add_modifier(Modifier::DIM),
                    ),
                    inner,
                );
                frame.render_widget(Paragraph::new(separator(12, styles.muted)), inner);
            })
            .unwrap();
        let buffer = terminal.backend().buffer();
        let text = buffer
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(text.contains("Tasks"), "{text}");
        assert!(text.contains("Running"), "{text}");
        assert_eq!(buffer[(0, 0)].fg, Color::Cyan);
    }

    #[test]
    fn foundation_ui_primitives_distinguish_active_and_inactive_selection() {
        let styles = styles();
        let shell = PaneShell::new("Rows", false, styles);
        assert_eq!(
            shell.row_style(true, true),
            Style::default().bg(Color::Blue)
        );
        assert!(
            shell
                .row_style(true, false)
                .add_modifier
                .contains(Modifier::UNDERLINED)
        );
        assert_eq!(shell.table_header_style(), styles.table_header);
    }

    #[test]
    fn foundation_ui_primitives_bound_scroll_indicators() {
        assert_eq!(BoundedScrollIndicator::new(99, 5, 12).label(), "↑  8-12/12");
        assert_eq!(BoundedScrollIndicator::new(0, 5, 12).label(), " ↓ 1-5/12");
        assert_eq!(BoundedScrollIndicator::new(0, 5, 0).label(), "0/0");
        assert!(!BoundedScrollIndicator::new(0, 20, 4).is_scrollable());
        assert_eq!(
            BoundedScrollIndicator::new(0, 5, 12).title_label(Some(2), true, true),
            Some("3/12 · ↓ · rows 1–5".into())
        );
        assert_eq!(
            BoundedScrollIndicator::new(4, 5, 12).title_label(Some(6), true, true),
            Some("7/12 · ↑↓ · rows 5–9".into())
        );
        assert_eq!(
            BoundedScrollIndicator::new(99, 5, 12).title_label(Some(99), true, false),
            Some("12/12 · ^ · rows 8–12".into())
        );
        assert_eq!(
            BoundedScrollIndicator::new(0, 20, 4).title_label(Some(0), true, true),
            None
        );
    }

    #[test]
    fn foundation_ui_primitives_hide_only_lower_priority_columns() {
        let columns = [
            ResponsiveColumn {
                minimum_width: 10,
                priority: 0,
            },
            ResponsiveColumn {
                minimum_width: 8,
                priority: 2,
            },
            ResponsiveColumn {
                minimum_width: 7,
                priority: 1,
            },
        ];
        assert_eq!(responsive_columns(17, &columns), vec![true, false, true]);
        assert_eq!(responsive_columns(25, &columns), vec![true, true, true]);
        assert_eq!(responsive_columns(4, &columns), vec![true, false, false]);
    }

    #[test]
    fn foundation_ui_primitives_statuses_have_text_markers() {
        for tone in [
            StatusTone::Success,
            StatusTone::Warning,
            StatusTone::Error,
            StatusTone::Running,
            StatusTone::Pending,
            StatusTone::Accent,
            StatusTone::Muted,
            StatusTone::Info,
            StatusTone::Disabled,
        ] {
            let label = status_label(tone, "state", Style::default());
            assert!(label.content.contains("state"));
            assert!(!tone.marker().is_empty());
        }
        for kind in [
            StateKind::Empty,
            StateKind::Loading,
            StateKind::Unavailable,
            StateKind::Partial,
            StateKind::Error,
        ] {
            assert!(!kind.marker().is_empty());
        }
    }

    #[test]
    fn action_lists_align_shortcuts_and_keep_disabled_reasons_in_text() {
        let items = vec![
            ActionListItem {
                marker: "✓",
                label: "Open Logs".into(),
                shortcut: "l".into(),
                state: "Local".into(),
                enabled: true,
                details: Vec::new(),
            },
            ActionListItem {
                marker: "×",
                label: "Cancel active build".into(),
                shortcut: "c".into(),
                state: "Disabled".into(),
                enabled: false,
                details: vec!["Reason: No active build can be cancelled.".into()],
            },
        ];
        let plain = action_list_plain(&items, 48);
        assert!(
            plain.contains("Open Logs            [l] — Local"),
            "{plain}"
        );
        assert!(
            plain.contains("Cancel active build  [c] — Disabled"),
            "{plain}"
        );
        assert!(
            plain.contains("Reason: No active build can be cancelled."),
            "{plain}"
        );

        let styled = action_list(
            &items,
            32,
            ActionListStyles {
                enabled: Style::default(),
                disabled: Style::default().add_modifier(Modifier::DIM),
                shortcut: Style::default().add_modifier(Modifier::BOLD),
                detail: Style::default(),
            },
        );
        assert!(
            styled.lines[1].spans[0]
                .style
                .add_modifier
                .contains(Modifier::DIM)
        );
        assert!(
            styled
                .lines
                .iter()
                .any(|line| line.to_string().contains("[c]"))
        );
    }

    #[test]
    fn dialog_primitives_bound_geometry_and_keep_state_textual() {
        let dialog_styles = DialogStyles {
            base: Style::default().fg(Color::White),
            focused_border: Style::default().fg(Color::Cyan),
            heading: Style::default().add_modifier(Modifier::BOLD),
            selected: Style::default().bg(Color::Blue),
            disabled: Style::default().add_modifier(Modifier::DIM),
            validation: Style::default().fg(Color::Red),
            hint: Style::default().fg(Color::DarkGray),
            destructive: Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        };
        let shell = DialogShell::new("Remove workspace", DialogTone::Destructive, dialog_styles);
        let field = shell.field("Recipe", "busybox", 8, true, false);
        let disabled = shell.field("Deploy", "unavailable", 8, false, true);
        let validation = shell.validation(Some("target changed"));
        let controls = shell.controls(Some(("Enter", "Confirm destructive")), &[("Esc", "Cancel")]);
        assert!(field.to_string().contains("▶ Recipe"));
        assert!(disabled.to_string().contains("– Deploy"));
        assert!(
            validation
                .to_string()
                .contains("✕ Validation: target changed")
        );
        assert!(controls.to_string().contains("[Enter] Confirm destructive"));
        assert!(controls.to_string().contains("[Esc] Cancel"));
        assert_eq!(
            bounded_dialog_rect(Rect::new(7, 11, 80, 24), 110, 30),
            Rect::new(8, 12, 78, 22)
        );

        let mut terminal = Terminal::new(TestBackend::new(50, 8)).unwrap();
        terminal
            .draw(|frame| frame.render_widget(shell.block(), frame.area()))
            .unwrap();
        let output = terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>();
        assert!(output.contains("Warning · Remove workspace"));
        assert_eq!(terminal.backend().buffer()[(0, 0)].fg, Color::Cyan);
    }
