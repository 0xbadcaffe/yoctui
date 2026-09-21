pub(crate) fn availability_state_word(state: WorkspaceAvailabilityState) -> &'static str {
    match state {
        WorkspaceAvailabilityState::Available => "available",
        WorkspaceAvailabilityState::AvailableWithLimitations => "limited",
        WorkspaceAvailabilityState::Unavailable => "unavailable",
        WorkspaceAvailabilityState::Unsupported => "unsupported",
        WorkspaceAvailabilityState::Unknown => "unknown",
    }
}

#[allow(dead_code)]
pub(crate) fn render_dashboard_actions(
    frame: &mut Frame,
    app: &App,
    projection: &DashboardProjection<'_>,
    area: Rect,
) {
    let action = &projection.next_action;
    let palette = ThemePalette::for_app(app);
    let tone = if action.enabled {
        palette.role(palette.running, Modifier::BOLD)
    } else {
        palette.role(palette.warning, Modifier::BOLD)
    };
    let mut lines = vec![
        Line::styled("Recommended", palette.role(palette.heading, Modifier::BOLD)),
        Line::styled(
            format!(
                "{} [{}] — {}",
                action.label,
                action.shortcut,
                availability_state_word(action.state)
            ),
            tone,
        ),
    ];
    if let Some(reason) = action.reason.as_deref() {
        lines.push(Line::styled(
            format!("Reason: {reason}"),
            palette.role(palette.warning, Modifier::DIM),
        ));
    }
    lines.extend([
        Line::default(),
        Line::styled(
            "Common actions",
            palette.role(palette.heading, Modifier::BOLD),
        ),
        Line::from("[F2] Tasks  [l] Logs  [e] Errors  [F3] Recent work"),
        Line::from("[F8] Artifacts  [f] Favorites  [t] Terminals"),
        Line::from("[E] Environment  [M] Sstate readiness"),
    ]);
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::default().title("Next Action").borders(Borders::ALL))
            .wrap(Wrap { trim: false }),
        area,
    );
}

#[allow(dead_code)]
pub(crate) fn render_dashboard_attention(
    frame: &mut Frame,
    app: &App,
    projection: &DashboardProjection<'_>,
    area: Rect,
) {
    let palette = ThemePalette::for_app(app);
    let lines = if projection.failures.is_empty()
        && (app.build.warnings > 0 || app.build.errors > 0)
    {
        vec![Line::styled(
            format!(
                "! Build reports {} warning(s), {} error(s); no retained diagnostic rows. [e] opens diagnostics.",
                app.build.warnings, app.build.errors
            ),
            palette.role(palette.warning, Modifier::BOLD),
        )]
    } else if projection.failures.is_empty() {
        vec![Line::styled(
            "✓ No retained warnings or errors. [e] opens diagnostics.",
            palette.role(palette.success, Modifier::BOLD),
        )]
    } else {
        projection
            .failures
            .iter()
            .map(|entry| {
                let marker = if entry.severity == Severity::Error {
                    "✕"
                } else {
                    "!"
                };
                Line::styled(
                    format!(
                        "{marker} {}",
                        entry.message.lines().next().unwrap_or("diagnostic")
                    ),
                    severity_style(app, entry.severity),
                )
            })
            .collect()
    };
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .title(format!(
                        "Attention · {} retained · e review",
                        projection.failures.len()
                    ))
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
}

#[allow(dead_code)]
pub(crate) fn render_dashboard_recent_builds(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    now: SystemTime,
) {
    let palette = ThemePalette::for_app(app);
    let block = pane_block(app, "Recent Builds · Job History", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if app.is_offline() && !app.saved_builds.records.is_empty() {
        let rows = app
            .saved_builds
            .records
            .iter()
            .take(usize::from(inner.height.saturating_sub(1)))
            .map(|r| {
                Row::new([
                    r.target.clone(),
                    format!("{:?}", r.outcome),
                    format!("{} saved logs", r.logs.len()),
                ])
            });
        frame.render_widget(
            Table::new(
                rows,
                [
                    Constraint::Percentage(55),
                    Constraint::Percentage(25),
                    Constraint::Percentage(20),
                ],
            )
            .header(Row::new([
                "Saved build · F3 details",
                "Outcome",
                "Retained logs",
            ])),
            inner,
        );
        return;
    }
    let projection = app.command_center_projection_at(now);
    let rows = projection
        .dashboard
        .recent_work
        .iter()
        .copied()
        .take(usize::from(inner.height.saturating_sub(1)))
        .enumerate()
        .map(|(index, row)| {
            Row::new([
                Cell::from((index + 1).to_string()),
                job_history_cell(app, row, job_history::JobHistoryColumn::Context, now),
                job_history_cell(app, row, job_history::JobHistoryColumn::Status, now),
                job_history_cell(app, row, job_history::JobHistoryColumn::Elapsed, now),
            ])
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        frame.render_widget(
            Paragraph::new("No completed builds or jobs retained. B opens build options."),
            inner,
        );
    } else {
        frame.render_widget(
            Table::new(
                rows,
                [
                    Constraint::Length(3),
                    Constraint::Min(18),
                    Constraint::Length(14),
                    Constraint::Length(10),
                ],
            )
            .header(
                Row::new(["#", "Image / Operation", "Result", "Duration"])
                    .style(palette.role(palette.table_header, Modifier::BOLD)),
            ),
            inner,
        );
    }
}

#[allow(dead_code)]
pub(crate) fn render_dashboard_quick_actions(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let block = pane_block(app, "Quick Actions", false);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }
    if app.is_offline() {
        frame.render_widget(Paragraph::new("[E] Configure build environment    [F3] Saved build history\n\nStart the daemon from your initialized Yocto shell: yoctui daemon start\nYoctui retries the connection automatically.").wrap(Wrap { trim:true }),inner);
        return;
    }
    let columns = Layout::horizontal([Constraint::Ratio(1, 3); 3]).split(inner);
    let actions = [
        ("B", "Build image", "Start the selected BitBake target"),
        ("t", "Open terminal", "Use the initialized Yocto shell"),
        ("E", "Verify environment", "Check layers and configuration"),
    ];
    for (index, ((key, label, detail), column)) in
        actions.into_iter().zip(columns.iter().copied()).enumerate()
    {
        let borders = if index + 1 < columns.len() {
            Borders::RIGHT
        } else {
            Borders::NONE
        };
        frame.render_widget(
            Paragraph::new(vec![
                Line::from(vec![
                    Span::styled(
                        format!("[{key}] "),
                        palette.role(palette.informational, Modifier::BOLD),
                    ),
                    Span::styled(label, palette.role(palette.heading, Modifier::BOLD)),
                ]),
                Line::default(),
                Line::from(detail),
            ])
            .block(Block::default().borders(borders))
            .wrap(Wrap { trim: false }),
            column,
        );
    }
}
