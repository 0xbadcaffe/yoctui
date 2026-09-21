fn concept_task_inspector(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    now: SystemTime,
    rows: &[TaskRowRef<'_>],
) {
    let block = pane_block(app, "Inspector: Task", app.focus == FocusTarget::Inspector);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let sections = Layout::vertical([
        Constraint::Min(20),
        Constraint::Length(11),
        Constraint::Length(5),
    ])
    .split(inner);
    let selected = app.task_inspector(rows.get(app.task_progress_scroll).copied(), 0);
    let palette = ThemePalette::for_app(app);
    frame.render_widget(
        Paragraph::new(format!(
            "{}\n\n{}",
            task_inspector_primary(&selected),
            task_inspector_context(&selected, now)
        ))
        .wrap(Wrap { trim: false }),
        sections[0],
    );
    let actions = task_inspector_actions(app);
    frame.render_widget(
        Paragraph::new(action_list(
            &actions,
            sections[1].width,
            inspector_action_styles(app),
        ))
        .block(
            Block::default()
                .title("Actions")
                .title_style(palette.role(palette.heading, Modifier::BOLD))
                .borders(Borders::TOP)
                .border_style(palette.role(palette.inactive_border, Modifier::empty())),
        ),
        sections[1],
    );
    frame.render_widget(
        Paragraph::new(system_status_document(app, sections[2].width)).block(
            Block::default()
                .title("System Status")
                .title_style(palette.role(palette.heading, Modifier::BOLD))
                .borders(Borders::TOP)
                .border_style(palette.role(palette.inactive_border, Modifier::empty())),
        ),
        sections[2],
    );
}

fn concept_error_inspector(frame: &mut Frame, app: &App, area: Rect) {
    let block = pane_block(app, "Inspector: Error", app.focus == FocusTarget::Inspector);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let regions = Layout::vertical([Constraint::Min(12), Constraint::Length(10)]).split(inner);
    let details=app.logs.diagnostics().nth(app.error_selection).map_or_else(|| "No retained diagnostic selected.".into(),|entry| {
        format!("Severity    : {:?}\nTask        : {}\nRecipe      : {}\nCurrent build exit : {}\nFirst seen  : {}\nSource log  : {}\n\nMessage\n{}",
            entry.severity,entry.task.as_deref().unwrap_or("unavailable"),entry.recipe.as_deref().unwrap_or("unavailable"),
            app.build.exit_code.map_or_else(|| "unavailable".into(),|code|code.to_string()),clock_text(entry.timestamp),
            entry.path.as_ref().map_or_else(|| "unavailable".into(),|path|path.display().to_string()),entry.message)
    });
    frame.render_widget(
        Paragraph::new(details).wrap(Wrap { trim: false }),
        regions[0],
    );
    let palette = ThemePalette::for_app(app);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from("[Enter] Open matching log"),
            Line::from("[o] Open source log"),
            Line::from("[s] Cycle severity filter"),
            Line::from("[f] Pause / follow logs"),
            Line::from("[B] Rebuild options"),
            Line::default(),
            Line::styled(
                "Rebuild requires review and confirmation.",
                palette.role(palette.warning, Modifier::empty()),
            ),
        ])
        .block(
            Block::default()
                .title("Recovery Actions")
                .title_style(palette.role(palette.heading, Modifier::BOLD))
                .borders(Borders::TOP),
        )
        .wrap(Wrap { trim: false }),
        regions[1],
    );
}
