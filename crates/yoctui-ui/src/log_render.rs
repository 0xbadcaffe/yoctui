//! Log render.
use super::*;

pub(crate) fn list_or_none(values: &[String]) -> String {
    if values.is_empty() {
        "(none)".into()
    } else {
        values.join(", ")
    }
}

pub(crate) fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KiB", "MiB", "GiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

pub(crate) fn log_severity_label(severity: Severity) -> &'static str {
    match severity {
        Severity::Trace => "· Trace",
        Severity::Info => "i Info",
        Severity::Warning => "! Warning",
        Severity::Error => "✕ Error",
    }
}

pub(crate) fn compact_log_activity(app: &App, width: u16) -> String {
    let detailed = width >= 96;
    let mut segments = vec![if app.logs.follow {
        if detailed {
            "▶ Following"
        } else {
            "▶ Follow"
        }
        .to_owned()
    } else {
        "Ⅱ Paused".to_owned()
    }];
    if app.logs.filter.is_some()
        || app.logs.recipe_filter.is_some()
        || app.logs.task_filter.is_some()
        || app.logs.build_filter.is_some()
    {
        segments.push("◆ Filtered".into());
    }
    if app.logs.searching || !app.logs.query.is_empty() {
        segments.push(if detailed && !app.logs.query.is_empty() {
            format!("/ Search {}", app.logs.query)
        } else {
            "/ Search".into()
        });
    }
    if app.logs.dropped > 0 {
        segments.push(if detailed {
            format!(
                "! Evicted {} [W {} E {}]",
                app.logs.dropped, app.logs.dropped_warnings, app.logs.dropped_errors
            )
        } else {
            format!("! Evicted {}", app.logs.dropped)
        });
    }
    if detailed && app.logs.coalesced > 0 {
        segments.push(format!("↺ {} coalesced", app.logs.coalesced));
    }
    if !app.logs.bookmarks.is_empty() {
        segments.push(format!("★ {} bookmarked", app.logs.bookmarks.len()));
    }
    segments.join(" · ")
}

pub(crate) fn log_filter_chips(app: &App, width: u16) -> String {
    let source = app.logs.source_filter.as_ref().map_or("all", |_| "on");
    let time = app.logs.time_range.label();
    if width < 80 {
        return format!(
            "Filters R:{} T:{} B:{} S:{source} I:{time}",
            if app.logs.recipe_filter.is_some() {
                "on"
            } else {
                "all"
            },
            if app.logs.task_filter.is_some() {
                "on"
            } else {
                "all"
            },
            if app.logs.build_filter.is_some() {
                "on"
            } else {
                "all"
            },
        );
    }
    format!(
        "Filters R:{} · T:{} · B:{} · S:{} · I:{time}",
        app.logs.recipe_filter.as_deref().unwrap_or("all"),
        app.logs.task_filter.as_deref().unwrap_or("all"),
        app.logs.build_filter.as_deref().unwrap_or("all"),
        app.logs.source_filter.as_ref().map_or_else(
            || "all".into(),
            |path| path.file_name().map_or_else(
                || path.display().to_string(),
                |name| name.to_string_lossy().into()
            ),
        ),
    )
}

pub(crate) fn log_actions(
    app: &App,
    selected: Option<&yoctui_model::LogEntry>,
    width: u16,
) -> String {
    selected.map_or_else(
        || "Actions unavailable — no selected entry · E Export view".into(),
        |entry| {
            let bookmark = if app.logs.is_bookmarked(entry.id) {
                "Remove"
            } else {
                "Bookmark"
            };
            if width >= 96 {
                let mut value = format!(
                    "Actions C Copy entry · E Export view · m {bookmark} bookmark · [/] bookmarks"
                );
                if entry.path.is_some() {
                    value.push_str(" · o Open source log");
                }
                value
            } else if entry.path.is_some() {
                format!("Actions C Copy · o Open source log · E Export · m {bookmark} · [/] Marks")
            } else {
                format!("Actions C Copy · E Export · m {bookmark} · [/] Marks")
            }
        },
    )
}

pub(crate) fn case_insensitive_ranges(value: &str, query: &str) -> Vec<(usize, usize)> {
    if query.is_empty() {
        return Vec::new();
    }
    let folded_query = query.to_lowercase();
    if folded_query.is_empty() {
        return Vec::new();
    }
    let mut folded = String::new();
    let mut units = Vec::new();
    let mut characters = value.char_indices().peekable();
    while let Some((original_start, character)) = characters.next() {
        let original_end = characters.peek().map_or(value.len(), |(index, _)| *index);
        let folded_start = folded.len();
        folded.extend(character.to_lowercase());
        units.push((folded_start, folded.len(), original_start, original_end));
    }

    let mut ranges = Vec::<(usize, usize)>::new();
    let mut cursor = 0;
    while cursor <= folded.len() {
        let Some(relative) = folded[cursor..].find(&folded_query) else {
            break;
        };
        let folded_start = cursor + relative;
        let folded_end = folded_start + folded_query.len();
        let original_start = units
            .iter()
            .find(|(_, end, _, _)| *end > folded_start)
            .map(|(_, _, start, _)| *start);
        let original_end = units
            .iter()
            .rev()
            .find(|(start, _, _, _)| *start < folded_end)
            .map(|(_, _, _, end)| *end);
        if let (Some(start), Some(end)) = (original_start, original_end) {
            if let Some((_, previous_end)) = ranges.last_mut().filter(|(_, end)| *end >= start) {
                *previous_end = (*previous_end).max(end);
            } else {
                ranges.push((start, end));
            }
        }
        cursor = folded_end.max(folded_start.saturating_add(1));
    }
    ranges
}

pub(crate) fn log_search_spans(app: &App, message: &str) -> Vec<Span<'static>> {
    let ranges = case_insensitive_ranges(message, &app.logs.query);
    if ranges.is_empty() {
        return vec![Span::raw(message.to_owned())];
    }
    let palette = ThemePalette::for_app(app);
    let highlight = palette.role(palette.accent, Modifier::BOLD | Modifier::UNDERLINED);
    let mut spans = Vec::with_capacity(ranges.len().saturating_mul(2).saturating_add(1));
    let mut cursor = 0;
    for (start, end) in ranges {
        if cursor < start {
            spans.push(Span::raw(message[cursor..start].to_owned()));
        }
        spans.push(Span::styled(message[start..end].to_owned(), highlight));
        cursor = end;
    }
    if cursor < message.len() {
        spans.push(Span::raw(message[cursor..].to_owned()));
    }
    spans
}

pub(crate) fn selected_log_context(selected: Option<&yoctui_model::LogEntry>) -> String {
    selected.map_or_else(
        || "no selection".into(),
        |entry| match (entry.recipe.as_deref(), entry.task.as_deref()) {
            (Some(recipe), Some(task)) => format!("{recipe}:{task}"),
            (Some(recipe), None) => recipe.into(),
            (None, Some(task)) => task.into(),
            (None, None) => "global".into(),
        },
    )
}

pub(crate) fn log_workspace_tabs(view: LogWorkspaceView) -> String {
    match view {
        LogWorkspaceView::BitBake => "[▶ BitBake logs]  [  Yoctui diagnostics]".into(),
        LogWorkspaceView::Yoctui => "[  BitBake logs]  [▶ Yoctui diagnostics]".into(),
    }
}

pub(crate) fn logs(frame: &mut Frame, app: &App, area: Rect) {
    if app.log_workspace_view == LogWorkspaceView::Yoctui {
        internal_logs(frame, app, area);
        return;
    }
    let chunks = Layout::vertical([Constraint::Length(7), Constraint::Min(3)]).split(area);
    let log_area = chunks[1];
    let height = log_area.height.saturating_sub(3) as usize;
    let window = app.logs.window(height);
    let selection = window.selection;
    let start = window.start;
    let visible = &window.entries;
    let selected = selection
        .checked_sub(start)
        .and_then(|offset| visible.get(offset).copied());
    let mode = format!(
        "{}  ·  {}  ·  wrap {}  ·  severity {}",
        log_workspace_tabs(app.log_workspace_view),
        compact_log_activity(app, 40),
        if app.logs.wrap { "on" } else { "off" },
        app.logs
            .filter
            .map_or_else(|| "all".into(), |severity| format!("{severity:?}"))
    );
    let content_width = chunks[0].width.saturating_sub(2);
    let filters = log_filter_chips(app, content_width);
    let pressure = if app.logs.dropped > 0 || app.logs.coalesced > 0 {
        format!(
            "{} evicted [W {} E {}], {} coalesced; retained {}/{} bytes",
            app.logs.dropped,
            app.logs.dropped_warnings,
            app.logs.dropped_errors,
            app.logs.coalesced,
            app.logs.retained_bytes,
            app.logs.max_bytes
        )
    } else {
        format!(
            "No eviction or coalescing; retained {}/{} bytes",
            app.logs.retained_bytes, app.logs.max_bytes
        )
    };
    let search = search_line(
        app,
        &app.logs.query,
        app.logs.searching,
        (window.total > 0).then_some(selection),
        window.total,
        SearchNavigation::Matches,
        SearchExit::Done,
        chunks[0].width.saturating_sub(2),
    );
    let actions = log_actions(app, selected, content_width);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(bounded_cell_text(&mode, content_width)),
            Line::from(bounded_cell_text(&actions, content_width)),
            Line::from(bounded_cell_text(&filters, content_width)),
            search,
            Line::from(bounded_cell_text(&pressure, content_width)),
        ])
        .block(pane_block(
            app,
            "Log activity",
            app.focus == FocusTarget::Workspace,
        )),
        chunks[0],
    );
    let vertical = if window.total == 0 {
        "0/0".into()
    } else {
        format!("{}/{total}", selection + 1, total = window.total)
    };
    let horizontal_offset = app
        .logs
        .horizontal_offset
        .min(window.maximum_horizontal_offset);
    let horizontal_maximum = window.maximum_horizontal_offset;
    let horizontal = if app.logs.wrap {
        "wrapped".into()
    } else {
        format!("{horizontal_offset}/{horizontal_maximum}")
    };
    let title = format!(
        "Log Viewer — {} · {} · V {vertical} · H {horizontal}",
        selected_log_context(selected),
        if app.logs.follow {
            "following"
        } else {
            "paused"
        },
    );
    let block = pane_block(app, &title, app.focus == FocusTarget::Workspace);
    if window.total == 0 {
        let inner = block.inner(log_area);
        frame.render_widget(block, log_area);
        let state = StateView {
            kind: StateKind::Empty,
            summary: if app.logs.entries.is_empty() {
                "No retained log entries.".into()
            } else {
                "No log entries match the active filters or search.".into()
            },
            detail: Some("Adjust filters or resume follow as needed.".into()),
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
    if app.logs.wrap {
        let mut lines = Vec::new();
        for (offset, log) in visible.iter().enumerate() {
            let selected = start + offset == selection;
            for (line_index, message) in log.message.lines().enumerate() {
                let prefix = if line_index == 0 {
                    format!(
                        "{} {} {} {} ",
                        if selected { "▶" } else { " " },
                        if app.logs.is_bookmarked(log.id) {
                            "★"
                        } else {
                            " "
                        },
                        log_severity_label(log.severity),
                        log.recipe.as_deref().unwrap_or("")
                    )
                } else {
                    "     ".into()
                };
                let style = if selected {
                    selected_log_style(app, log.severity)
                } else {
                    severity_style(app, log.severity)
                };
                let mut spans = vec![Span::raw(prefix)];
                spans.extend(log_search_spans(app, message));
                lines.push(Line::from(spans).style(style));
            }
        }
        yocto_logs::render(frame, log_area, Text::from(lines), block, true);
        return;
    }
    let rows = visible.iter().enumerate().map(|(offset, l)| {
        let selected = start + offset == selection;
        let message = l
            .message
            .chars()
            .skip(horizontal_offset)
            .collect::<String>();
        (
            vec![
                Line::from(if app.logs.is_bookmarked(l.id) {
                    format!("★ {}", log_severity_label(l.severity))
                } else {
                    log_severity_label(l.severity).into()
                }),
                Line::from(l.recipe.as_deref().unwrap_or("")),
                Line::from(l.task.as_deref().unwrap_or("")),
                Line::from(log_search_spans(app, &message)),
            ],
            if selected {
                selected_log_style(app, l.severity)
            } else {
                severity_style(app, l.severity)
            },
        )
    });
    yocto_logs::table(
        frame,
        log_area,
        block,
        &[
            Constraint::Length(11),
            Constraint::Length(16),
            Constraint::Length(18),
            Constraint::Min(10),
        ],
        &["Level", "Recipe", "Task", "Message"],
        rows,
    );
}
