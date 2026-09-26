//! Error render.
use super::*;

pub(crate) fn errors(frame: &mut Frame, app: &App, area: Rect) {
    if let Some(viewer) = &app.error_workspace.viewer {
        render_error_log_viewer(frame, app, area, viewer);
        return;
    }
    let [tabs, body] = Layout::vertical([Constraint::Length(3), Constraint::Min(1)]).areas(area);
    let current = app.error_workspace.view == yoctui_model::ErrorWorkspaceView::Current;
    let history = yoctui_model::historical_errors(&app.saved_builds.records);
    frame.render_widget(
        Paragraph::new(format!(
            "{} 1 Current build ({})    {} 2 Past builds ({})    Tab switches",
            if current { "▶" } else { " " },
            app.logs.diagnostics().count(),
            if current { " " } else { "▶" },
            history.len(),
        ))
        .style(Style::default().add_modifier(Modifier::BOLD))
        .block(Block::default().title("Build errors").borders(Borders::ALL)),
        tabs,
    );
    if current {
        current_errors(frame, app, body);
    } else {
        historical_error_view(frame, app, body, &history);
    }
}

fn current_errors(frame: &mut Frame, app: &App, area: Rect) {
    let errors = app.logs.diagnostics().collect::<Vec<_>>();
    let selected = errors.get(app.error_selection).copied();

    if area.height >= 32 {
        let chunks = Layout::vertical([
            Constraint::Length(5),
            Constraint::Length(8),
            Constraint::Min(10),
            Constraint::Length(5),
        ])
        .split(area);
        let target = app.build.target.as_deref().unwrap_or("not selected");
        let exit = app
            .build
            .exit_code
            .map_or_else(|| "unavailable".into(), |code| code.to_string());
        let selected_context = selected.map_or_else(
            || "no diagnostic selected".into(),
            |entry| {
                format!(
                    "{}:{}",
                    entry.recipe.as_deref().unwrap_or("unknown recipe"),
                    entry.task.as_deref().unwrap_or("unknown task")
                )
            },
        );
        frame.render_widget(
            Paragraph::new(format!(
                "Target: {target}  ·  Result: {:?} (exit {exit})\nDiagnostics: {} error / {} warning  ·  Selected: {selected_context}",
                app.build.status, app.build.errors, app.build.warnings,
            ))
            .block(
                Block::default()
                    .title(format!("Build Result · {}",app.build.status))
                    .title_style(build_status_style(app))
                    .borders(Borders::ALL),
            ),
            chunks[0],
        );

        let checked_for = |severity| {
            if app.logs.filter.is_none_or(|active| active == severity) {
                yoctui_model::CheckboxValue::Checked
            } else {
                yoctui_model::CheckboxValue::Unchecked
            }
        };
        let mut warning_filter = yoctui_model::CheckboxState::new("warnings", "Warnings");
        warning_filter.value = checked_for(Severity::Warning);
        let mut error_filter = yoctui_model::CheckboxState::new("errors", "Errors");
        error_filter.value = checked_for(Severity::Error);
        error_filter.focused = true;
        let mut related_filter =
            yoctui_model::CheckboxState::new("related", "Related task context");
        related_filter.value = if selected.is_some() {
            yoctui_model::CheckboxValue::Indeterminate
        } else {
            yoctui_model::CheckboxValue::Unchecked
        };
        let unicode = app.preferences.symbols == SymbolPreference::Unicode;
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(format!(
                    "{}   {}",
                    checkbox_text(&error_filter, unicode),
                    checkbox_text(&warning_filter, unicode),
                )),
                Line::from(format!(
                    "{}  ·  s cycles severity",
                    checkbox_text(&related_filter, unicode),
                )),
            ])
            .block(
                Block::default()
                    .title("Correlated-log filters")
                    .borders(Borders::ALL),
            ),
            chunks[3],
        );

        render_error_table(frame, app, chunks[1], &errors);
        render_correlated_error_log(frame, app, chunks[2], selected);
        return;
    }

    let chunks = Layout::vertical([Constraint::Min(4), Constraint::Length(12)]).split(area);
    render_error_table(frame, app, chunks[0], &errors);
    let detail = selected.map_or_else(
        || "No retained warnings or errors.".into(),
        |log| diagnostic_detail(app, log),
    );
    frame.render_widget(
        Paragraph::new(format!(
            "{detail}\n\nEnter views the source log.  l jumps to matching live logs.  o opens the source externally."
        ))
        .block(
            Block::default()
                .title("Selected diagnostic")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false }),
        chunks[1],
    );
}

fn historical_error_view(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    errors: &[yoctui_model::HistoricalError<'_>],
) {
    let [table_area, detail_area] =
        Layout::vertical([Constraint::Percentage(55), Constraint::Percentage(45)]).areas(area);
    let selection = app
        .error_workspace
        .history_selection
        .min(errors.len().saturating_sub(1));
    let viewport = yoctui_model::centered_viewport_range(
        (!errors.is_empty()).then_some(selection),
        errors.len(),
        usize::from(table_area.height.saturating_sub(3)).max(1),
    );
    let rows = errors[viewport.clone()]
        .iter()
        .enumerate()
        .map(|(offset, entry)| {
            let index = viewport.start + offset;
            let status = match entry.build.outcome {
                yoctui_model::SavedBuildOutcome::Failed if entry.resolved => "Resolved",
                yoctui_model::SavedBuildOutcome::Failed => "Unresolved",
                yoctui_model::SavedBuildOutcome::Succeeded => "Succeeded",
                yoctui_model::SavedBuildOutcome::Cancelled => "Cancelled",
                yoctui_model::SavedBuildOutcome::Lost => "Lost",
                yoctui_model::SavedBuildOutcome::Incomplete => "Incomplete",
            };
            Row::new([
                clock_text(UNIX_EPOCH + Duration::from_millis(entry.log.unix_ms)),
                status.into(),
                entry.build.target.clone(),
                entry.log.recipe.clone().unwrap_or_default(),
                entry.log.task.clone().unwrap_or_default(),
                entry
                    .log
                    .message
                    .lines()
                    .next()
                    .unwrap_or_default()
                    .to_owned(),
            ])
            .style(if index == selection {
                selected_log_style(app, entry.log.severity)
            } else {
                severity_style(app, entry.log.severity)
            })
        });
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Length(8),
                Constraint::Length(11),
                Constraint::Length(24),
                Constraint::Length(16),
                Constraint::Length(16),
                Constraint::Min(18),
            ],
        )
        .header(
            Row::new(["Time", "State", "Target", "Recipe", "Task", "Summary"])
                .style(Style::default().bold()),
        )
        .block(
            Block::default()
                .title("Saved errors and warnings · newest build first")
                .borders(Borders::ALL),
        ),
        table_area,
    );
    let detail = errors.get(selection).map_or_else(
        || "No saved warning or error records are available.".into(),
        |entry| {
            format!(
                "Target: {}\nMachine: {}\nOutcome: {:?}{}\nRecipe: {}  Task: {}\nSource log: {}\nSaved build: {}\n\n{}\n\nEnter view log · o external editor{}",
                entry.build.target,
                entry.build.machine.as_deref().unwrap_or("not recorded"),
                entry.build.outcome,
                if entry.resolved { " · Resolved by a newer successful build" } else { "" },
                entry.log.recipe.as_deref().unwrap_or("not recorded"),
                entry.log.task.as_deref().unwrap_or("not recorded"),
                entry.log.path.as_deref().unwrap_or("not retained"),
                entry.build.id,
                entry.log.message,
                if entry.resolved { " · d/Delete remove resolved history" } else { "" },
            )
        },
    );
    frame.render_widget(
        Paragraph::new(detail).wrap(Wrap { trim: false }).block(
            Block::default()
                .title("Saved diagnostic")
                .borders(Borders::ALL),
        ),
        detail_area,
    );
}

fn render_error_log_viewer(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    viewer: &yoctui_model::ErrorLogViewer,
) {
    let status = if viewer.loading {
        "Loading source log…"
    } else if viewer.error.is_some() {
        "Source file unavailable; showing retained diagnostic"
    } else if viewer.path.is_some() {
        "Read-only source log"
    } else {
        "Retained diagnostic"
    };
    let mut lines = vec![Line::from(status).style(Style::default().bold())];
    if let Some(error) = &viewer.error {
        lines.push(Line::from(format!("Read error: {error}")));
        lines.push(Line::from(""));
    }
    lines.extend(
        viewer
            .content
            .lines()
            .skip(viewer.scroll)
            .map(|line| Line::from(line.to_owned())),
    );
    let total = viewer.content.lines().count().max(1);
    let visible = usize::from(area.height.saturating_sub(4)).max(1);
    let scroll = BoundedScrollIndicator::new(viewer.scroll, visible, total).label();
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }).block(
            Block::default()
                .title(format!("{} · {} · Esc back", viewer.title, scroll))
                .title_style(
                    Style::default()
                        .fg(ThemePalette::for_app(app).accent)
                        .add_modifier(Modifier::BOLD),
                )
                .borders(Borders::ALL),
        ),
        area,
    );
}

pub(crate) fn render_error_table(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    errors: &[&yoctui_model::LogEntry],
) {
    let height = area.height.saturating_sub(3) as usize;
    let selection = app.error_selection.min(errors.len().saturating_sub(1));
    let end = selection.saturating_add(1).max(height).min(errors.len());
    let start = end.saturating_sub(height);
    let rows = errors[start..end].iter().enumerate().map(|(offset, log)| {
        let index = start + offset;
        let diagnostic = log.diagnostic.as_ref();
        Row::new(vec![
            Cell::from(clock_text(log.timestamp)),
            Cell::from(format!("{:?}", log.severity)),
            Cell::from(log.recipe.as_deref().unwrap_or("")),
            Cell::from(log.task.as_deref().unwrap_or("")),
            Cell::from(diagnostic.map_or(log.message.as_str(), |value| value.summary.as_str())),
            Cell::from(log.build.as_deref().unwrap_or("")),
        ])
        .style(if index == selection {
            selected_log_style(app, log.severity)
        } else {
            severity_style(app, log.severity)
        })
    });
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Length(8),
                Constraint::Length(9),
                Constraint::Length(14),
                Constraint::Length(16),
                Constraint::Min(18),
                Constraint::Length(if area.width >= 110 { 20 } else { 0 }),
            ],
        )
        .header(
            Row::new(["Time", "Severity", "Recipe", "Task", "Summary", "Build"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(format!(
                    "Errors and warnings ({} retained; {} warning / {} error records evicted)",
                    errors.len(),
                    app.logs.dropped_warnings,
                    app.logs.dropped_errors,
                ))
                .borders(Borders::ALL),
        ),
        area,
    );
}

pub(crate) fn render_correlated_error_log(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    selected: Option<&yoctui_model::LogEntry>,
) {
    let panes = Layout::vertical([Constraint::Min(5), Constraint::Length(4)]).split(area);
    let visible_len = app.logs.paused_len.unwrap_or(app.logs.entries.len());
    let correlated = app
        .logs
        .entries
        .iter()
        .take(visible_len)
        .filter(|entry| {
            selected.is_some_and(|diagnostic| {
                diagnostic
                    .build
                    .as_ref()
                    .is_none_or(|build| entry.build.as_ref() == Some(build))
                    && diagnostic
                        .recipe
                        .as_ref()
                        .is_none_or(|recipe| entry.recipe.as_ref() == Some(recipe))
                    && diagnostic
                        .task
                        .as_ref()
                        .is_none_or(|task| entry.task.as_ref() == Some(task))
            })
        })
        .collect::<Vec<_>>();
    let query = app.logs.query.to_ascii_lowercase();
    let matches = correlated
        .iter()
        .filter(|entry| {
            app.logs
                .filter
                .is_none_or(|severity| entry.severity == severity)
                && (query.is_empty() || entry.message.to_ascii_lowercase().contains(&query))
        })
        .collect::<Vec<_>>();
    let match_position = (!matches.is_empty()).then_some(matches.len());
    let pause = if app.logs.follow {
        "Following"
    } else {
        "Paused"
    };
    let match_label = match_position.map_or_else(
        || "match 0/0".into(),
        |position| format!("match {position}/{}", matches.len()),
    );
    let viewport = panes[0].height.saturating_sub(3) as usize;
    let start = matches.len().saturating_sub(viewport);
    let scroll = BoundedScrollIndicator::new(start, viewport, matches.len()).label();
    let title = format!(
        "Correlated · {pause} · {match_label} · loss {} W{} E{} · {scroll}",
        app.logs.dropped, app.logs.dropped_warnings, app.logs.dropped_errors,
    );
    let rows = matches[start..].iter().map(|entry| {
        (
            vec![
                Line::from(clock_text(entry.timestamp)),
                Line::from(log_severity_label(entry.severity)),
                Line::from(entry.task.as_deref().unwrap_or("")),
                Line::from(entry.message.as_str()),
            ],
            severity_style(app, entry.severity),
        )
    });
    yocto_logs::table(
        frame,
        panes[0],
        Block::default().title(title).borders(Borders::ALL),
        &[
            Constraint::Length(10),
            Constraint::Length(9),
            Constraint::Length(13),
            Constraint::Min(12),
        ],
        &["Time", "Severity", "Task", "Message"],
        rows,
    );
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(
                "Enter view source log · l matching live log · o external · B rebuild options",
            ),
            Line::from("Rebuild: review + confirmation required."),
        ])
        .block(
            Block::default()
                .title("Recovery actions")
                .borders(Borders::ALL),
        ),
        panes[1],
    );
}

pub(crate) fn diagnostic_detail(app: &App, log: &yoctui_model::LogEntry) -> String {
    let diagnostic = log.diagnostic.as_ref();
    let metadata = diagnostic.map_or_else(
        || "unavailable".into(),
        |value| {
            value
                .event_metadata
                .iter()
                .map(|(name, value)| format!("{name}={value}"))
                .collect::<Vec<_>>()
                .join(", ")
        },
    );
    let suggestions = diagnostic.map_or_else(
        || "No suggested actions are available.".into(),
        |value| value.suggestions.join("\n- "),
    );
    let related = app
        .logs
        .diagnostics()
        .filter(|candidate| {
            candidate.id != log.id
                && ((log.recipe.is_some() && candidate.recipe == log.recipe)
                    || (log.task.is_some() && candidate.task == log.task)
                    || (log.build.is_some() && candidate.build == log.build))
        })
        .filter_map(|candidate| candidate.diagnostic.as_ref())
        .map(|value| value.summary.as_str())
        .take(3)
        .collect::<Vec<_>>();
    format!(
        "Category: {}\nSummary: {}\nTime: {}\nBuild: {}\nRecipe: {}  Task: {}\nSource log: {}\nEvent metadata: {}\n\nFull message:\n{}\n\nSuggested actions:\n- {}\n\nRelated diagnostics:\n{}",
        diagnostic.map_or("unavailable", |value| value.category.as_str()),
        diagnostic.map_or("unavailable", |value| value.summary.as_str()),
        clock_text(log.timestamp),
        log.build.as_deref().unwrap_or("unavailable"),
        log.recipe.as_deref().unwrap_or("unavailable"),
        log.task.as_deref().unwrap_or("unavailable"),
        log.path
            .as_ref()
            .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
        metadata,
        log.message,
        suggestions,
        if related.is_empty() {
            "none".into()
        } else {
            related.join("\n")
        },
    )
}
