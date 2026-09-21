pub fn render_bar_chart(
    frame: &mut ratatui::Frame,
    area: Rect,
    projection: &BarProjection,
    styles: WidgetStyles,
    options: WidgetRenderOptions,
) {
    if area.is_empty() {
        return;
    }
    if projection.values.is_empty() || projection.state != WidgetState::Available {
        let text = bounded_explicit_state_text(
            projection.state,
            projection.detail.as_deref(),
            options,
            area.width,
        );
        frame.render_widget(
            Paragraph::new(text).style(styles.state(projection.state, WidgetRole::Primary)),
            area,
        );
        return;
    }
    if !options.unicode || area.height < 3 {
        let lines = projection
            .values
            .iter()
            .take(usize::from(area.height))
            .map(|bar| {
                Line::styled(
                    bounded_line_text(
                        &format!("{}: {}", bar.label, bar.value),
                        area.width,
                        options.unicode,
                    ),
                    styles.role(bar.role),
                )
            })
            .collect::<Vec<_>>();
        frame.render_widget(Paragraph::new(lines), area);
        return;
    }
    let bars = projection
        .values
        .iter()
        .map(|bar| {
            Bar::with_label(bar.label.clone(), bar.value)
                .style(styles.role(bar.role))
                .value_style(styles.role(bar.role).add_modifier(Modifier::BOLD))
                .text_value(bar.value.to_string())
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        BarChart::new(bars)
            .bar_width(3)
            .bar_gap(1)
            .bar_set(symbols::bar::NINE_LEVELS)
            .value_style(styles.primary)
            .label_style(styles.primary),
        area,
    );
}

pub fn render_tabs(
    frame: &mut ratatui::Frame,
    area: Rect,
    projection: &TabProjection,
    styles: WidgetStyles,
    options: WidgetRenderOptions,
) {
    if area.is_empty() {
        return;
    }
    if projection.labels.is_empty() {
        frame.render_widget(
            Paragraph::new(explicit_state_text(WidgetState::Empty, None, options))
                .style(styles.disabled),
            area,
        );
        return;
    }
    let titles = projection
        .labels
        .iter()
        .enumerate()
        .map(|(index, label)| {
            if Some(index) == projection.selected {
                format!("[{label}]")
            } else {
                label.clone()
            }
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Tabs::new(titles)
            .select(projection.selected)
            .divider(if options.unicode { "│" } else { "|" })
            .style(styles.primary)
            .highlight_style(styles.selected.add_modifier(Modifier::BOLD)),
        area,
    );
}

pub fn render_legend(
    frame: &mut ratatui::Frame,
    area: Rect,
    projection: &LegendProjection,
    styles: WidgetStyles,
    options: WidgetRenderOptions,
) {
    if area.is_empty() {
        return;
    }
    if projection.items.is_empty() || projection.state != WidgetState::Available {
        let text = bounded_explicit_state_text(
            projection.state,
            projection.detail.as_deref(),
            options,
            area.width,
        );
        frame.render_widget(
            Paragraph::new(text).style(styles.state(projection.state, WidgetRole::Primary)),
            area,
        );
        return;
    }
    let marker = if options.unicode { "■" } else { "#" };
    let lines = projection
        .items
        .iter()
        .take(usize::from(area.height))
        .map(|item| {
            Line::from(vec![
                Span::styled(format!("{marker} "), styles.role(item.role)),
                Span::styled(
                    bounded_line_text(
                        &format!("{}: {}", item.label, item.value),
                        area.width.saturating_sub(2),
                        options.unicode,
                    ),
                    styles.primary,
                ),
            ])
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), area);
}

pub fn render_scrollbar(
    frame: &mut ratatui::Frame,
    area: Rect,
    projection: ScrollbarProjection,
    styles: WidgetStyles,
    options: WidgetRenderOptions,
) {
    if area.is_empty() {
        return;
    }
    let label = projection.label();
    if projection.state == WidgetState::Empty || area.height == 1 {
        frame.render_widget(
            Paragraph::new(bounded_line_text(&label, area.width, options.unicode)).style(
                if projection.state == WidgetState::Empty {
                    styles.disabled
                } else {
                    styles.primary
                },
            ),
            area,
        );
        return;
    }
    let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area);
    let mut state = ScrollbarState::new(projection.scroll.total)
        .position(projection.scroll.offset)
        .viewport_content_length(projection.scroll.viewport);
    let scrollbar = if options.unicode {
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
    } else {
        Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .thumb_symbol("#")
            .track_symbol(Some("|"))
            .begin_symbol(Some("^"))
            .end_symbol(Some("v"))
    };
    frame.render_stateful_widget(scrollbar.style(styles.accent), rows[0], &mut state);
    frame.render_widget(
        Paragraph::new(bounded_line_text(&label, rows[1].width, options.unicode))
            .style(styles.primary),
        rows[1],
    );
}
