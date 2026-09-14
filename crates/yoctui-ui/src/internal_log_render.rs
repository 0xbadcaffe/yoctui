//! Internal log render.
use super::*;

pub(crate) fn internal_log_level_label(level: InternalLogLevel) -> &'static str {
    match level {
        InternalLogLevel::Trace => "· Trace",
        InternalLogLevel::Debug => "◇ Debug",
        InternalLogLevel::Info => "i Info",
        InternalLogLevel::Warning => "! Warning",
        InternalLogLevel::Error => "✕ Error",
    }
}

pub(crate) fn internal_log_severity(level: InternalLogLevel) -> Severity {
    match level {
        InternalLogLevel::Trace | InternalLogLevel::Debug => Severity::Trace,
        InternalLogLevel::Info => Severity::Info,
        InternalLogLevel::Warning => Severity::Warning,
        InternalLogLevel::Error => Severity::Error,
    }
}

pub(crate) fn internal_log_search_spans(app: &App, message: &str) -> Vec<Span<'static>> {
    let ranges = case_insensitive_ranges(message, &app.internal_logs.query);
    if ranges.is_empty() {
        return vec![Span::raw(message.to_owned())];
    }
    let palette = ThemePalette::for_app(app);
    let hit_style = if app.color_enabled {
        palette.role(palette.accent, Modifier::BOLD | Modifier::UNDERLINED)
    } else {
        Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
    };
    let mut spans = Vec::new();
    let mut cursor = 0;
    for (start, end) in ranges {
        if cursor < start {
            spans.push(Span::raw(message[cursor..start].to_owned()));
        }
        spans.push(Span::styled(message[start..end].to_owned(), hit_style));
        cursor = end;
    }
    if cursor < message.len() {
        spans.push(Span::raw(message[cursor..].to_owned()));
    }
    spans
}

pub(crate) fn internal_logs(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::vertical([Constraint::Length(7), Constraint::Min(3)]).split(area);
    let list_area = chunks[1];
    let content_width = chunks[0].width.saturating_sub(2);
    let height = list_area.height.saturating_sub(3) as usize;
    let window = app.internal_logs.window(height);
    let selection = window.selection;
    let mode = format!(
        "{}  ·  {}  ·  {} retained",
        log_workspace_tabs(app.log_workspace_view),
        if app.internal_logs.follow {
            "▶ Following"
        } else {
            "Ⅱ Paused"
        },
        app.internal_logs.entries.len()
    );
    let actions = "Actions E Export diagnostics · c Clear retained diagnostics";
    let filters = format!(
        "Filters s level: {} · T target: {}",
        app.internal_logs
            .level_filter
            .map_or("all", InternalLogLevel::label),
        app.internal_logs.target_filter.as_deref().unwrap_or("all")
    );
    let search = search_line(
        app,
        &app.internal_logs.query,
        app.internal_logs.searching,
        (window.total > 0).then_some(selection),
        window.total,
        SearchNavigation::Results,
        SearchExit::Done,
        content_width,
    );
    let pressure = format!(
        "Retention evicted {} · ingress dropped {} · retained {}/{} bytes",
        app.internal_logs.evicted,
        app.internal_logs.ingress_dropped,
        app.internal_logs.retained_bytes,
        app.internal_logs.max_bytes
    );
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(bounded_cell_text(&mode, content_width)),
            Line::from(bounded_cell_text(actions, content_width)),
            Line::from(bounded_cell_text(&filters, content_width)),
            search,
            Line::from(bounded_cell_text(&pressure, content_width)),
        ])
        .block(pane_block(
            app,
            "Yoctui self-diagnostic activity",
            app.focus == FocusTarget::Workspace,
        )),
        chunks[0],
    );

    let position = app.internal_logs.vertical_position().map_or_else(
        || "0/0".into(),
        |(current, total)| format!("{current}/{total}"),
    );
    let title = format!(
        "Yoctui diagnostics — local tracing · {} · {position}",
        if app.internal_logs.follow {
            "following"
        } else {
            "paused"
        }
    );
    let block = pane_block(app, &title, app.focus == FocusTarget::Workspace);
    if window.total == 0 {
        let inner = block.inner(list_area);
        frame.render_widget(block, list_area);
        let state = StateView {
            kind: StateKind::Empty,
            summary: if app.internal_logs.entries.is_empty() {
                "No Yoctui self-diagnostics retained.".into()
            } else {
                "No Yoctui diagnostics match the active filters or search.".into()
            },
            detail: Some(
                "This local tracing authority is separate from BitBake Logs; v switches views."
                    .into(),
            ),
            action: None,
        };
        let palette = ThemePalette::for_app(app);
        frame.render_widget(
            state.paragraph(
                palette.role(palette.muted, Modifier::BOLD),
                palette.role(palette.secondary_foreground, Modifier::DIM),
            ),
            inner,
        );
        return;
    }

    let rows = window.entries.iter().enumerate().map(|(offset, entry)| {
        let selected = window.start + offset == selection;
        let severity = internal_log_severity(entry.level);
        Row::new([
            Cell::from(timestamp_text(entry.timestamp)),
            Cell::from(internal_log_level_label(entry.level)),
            Cell::from(entry.target.as_str()),
            Cell::from(Line::from(internal_log_search_spans(app, &entry.message))),
        ])
        .style(if selected {
            selected_log_style(app, severity)
        } else {
            severity_style(app, severity)
        })
    });
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Length(14),
                Constraint::Length(11),
                Constraint::Length(24),
                Constraint::Min(10),
            ],
        )
        .header(
            Row::new(["Time", "Level", "Target", "Message"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(block),
        list_area,
    );
}
