pub(crate) fn render_build_summary(frame: &mut Frame, app: &App, area: Rect, now: SystemTime) {
    if area.is_empty() {
        return;
    }
    let summary = app.build_summary_at(now);
    let progress = app.progress_hierarchy_at(now);
    let rows = if app.screen == Screen::Dashboard && area.height >= 4 {
        Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area)
    } else if area.height >= 2 {
        Layout::vertical([Constraint::Length(1), Constraint::Length(1)]).split(area)
    } else {
        Layout::vertical([Constraint::Length(1)]).split(area)
    };
    let palette = ThemePalette::for_app(app);
    if let Some(fraction) = progress.build.fraction {
        let percent = fraction.percent();
        let total = fraction.total;
        let label = format!(
            "Overall  {percent}%  {}/{}",
            fraction.current.min(total),
            total
        );
        if app.preferences.symbols == SymbolPreference::Ascii {
            frame.render_widget(
                Paragraph::new(format!("{label}  {}", task_progress_bar(app, percent)))
                    .style(build_status_style(app)),
                rows[0],
            );
        } else {
            frame.render_widget(
                ratatui::widgets::Gauge::default()
                    .gauge_style(build_status_style(app).bg(palette.inactive_border))
                    .ratio(f64::from(percent) / 100.0)
                    .use_unicode(true)
                    .label(label),
                rows[0],
            );
        }
    } else if matches!(app.build.status, BuildStatus::Idle)
        && summary.completed == 0
        && app.tasks.is_empty()
    {
        frame.render_widget(
            Paragraph::new("Overall  build not started · 0%")
                .style(palette.role(palette.pending, Modifier::BOLD)),
            rows[0],
        );
    } else {
        frame.render_widget(
            Paragraph::new(format!(
                "Overall  progress unknown {}  {}/—",
                task_activity(app, None),
                summary.completed
            ))
            .style(
                palette
                    .role(palette.running, Modifier::BOLD)
                    .add_modifier(Modifier::BOLD),
            ),
            rows[0],
        );
    }
    if let Some(row) = rows.get(1) {
        let elapsed = summary.elapsed.map_or_else(|| "--".into(), format_duration);
        let text = if area.width >= 84 {
            let mut text = format!(
                "Active {}  Waiting {}  Warnings {}  Errors {}  Elapsed {elapsed}",
                summary.active, summary.waiting, summary.warnings, summary.errors
            );
            if area.width >= 96 {
                text.push_str("  ");
                text.push_str(&build_pace_at(app, now));
            }
            text
        } else {
            format!(
                "A{} W{} !{} ✕{} {elapsed}",
                summary.active, summary.waiting, summary.warnings, summary.errors
            )
        };
        frame.render_widget(
            Paragraph::new(text).style(palette.role(palette.secondary_foreground, Modifier::DIM)),
            *row,
        );
    }
    if let Some(row) = rows.get(2) {
        let exit = app
            .build
            .exit_code
            .map_or_else(|| "none".into(), |code| code.to_string());
        frame.render_widget(
            Paragraph::new(format!(
                "Target: {}  Backend: {}  Status: {}  Exit code: {exit}",
                app.build.target.as_deref().unwrap_or("not selected"),
                app.backend,
                app.build.status,
            )),
            *row,
        );
    }
    if let Some(row) = rows.get(3) {
        let parse = app.build.parse_current.map_or_else(
            || "not parsing".into(),
            |current| {
                app.build
                    .parse_total
                    .map_or_else(|| current.to_string(), |total| format!("{current}/{total}"))
            },
        );
        let machine = app
            .workspace
            .variables
            .get("MACHINE")
            .map_or("unknown", String::as_str);
        let distro = app
            .workspace
            .variables
            .get("DISTRO")
            .map_or("unknown", String::as_str);
        let cpu = app
            .host_telemetry
            .cpu_utilization_percent
            .map_or_else(|| "unavailable".into(), |value| format!("{value}%"));
        let disk = app
            .host_telemetry
            .disk_available_bytes
            .map_or_else(|| "unavailable".into(), format_bytes);
        let mut facts = format!(
            "Parse progress: {parse}  Tasks: {}/{}  Warnings: {}  Errors: {}  Release: {}",
            app.build.completed,
            app.build
                .total
                .map_or_else(|| "—".into(), |value| value.to_string()),
            app.build.warnings,
            app.build.errors,
            app.workspace.release.as_deref().unwrap_or("unknown"),
        );
        if area.width >= 100 {
            facts.push_str(&format!(
                "  Machine: {machine}  Distro: {distro}  Host CPU: {cpu}  Build disk free: {disk}"
            ));
        }
        frame.render_widget(Paragraph::new(facts), *row);
    }
}
