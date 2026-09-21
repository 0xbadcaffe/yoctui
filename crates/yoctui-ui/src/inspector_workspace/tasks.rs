pub(crate) fn tasks_inspector(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    now: SystemTime,
    task_rows: &[TaskRowRef<'_>],
) {
    if frame.area().height >= 50 && area.height >= 40 {
        concept_task_inspector(frame, app, area, now, task_rows);
        return;
    }
    let selected = task_rows.get(app.task_progress_scroll).copied();
    let focused = app.focus == FocusTarget::Inspector;
    if area.height < 35 {
        let inspector = app.task_inspector(selected, 0);
        let sections = Layout::vertical([Constraint::Min(10), Constraint::Length(7)]).split(area);
        frame.render_widget(
            Paragraph::new(format!(
                "{}\n{}",
                task_inspector_primary(&inspector),
                task_inspector_context(&inspector, now)
            ))
            .block(pane_block(app, "Inspector: Task", focused))
            .wrap(Wrap { trim: false }),
            sections[0],
        );
        let actions = task_inspector_actions(app);
        frame.render_widget(
            Paragraph::new(action_list(
                &actions,
                sections[1].width.saturating_sub(2),
                inspector_action_styles(app),
            ))
            .block(pane_block(app, "Contextual Actions", false)),
            sections[1],
        );
        return;
    }

    let show_system = area.height >= 40;
    let sections = if area.height == 40 {
        Layout::vertical([
            Constraint::Length(10),
            Constraint::Length(10),
            Constraint::Length(6),
            Constraint::Length(8),
            Constraint::Length(6),
        ])
        .split(area)
    } else if show_system {
        Layout::vertical([
            Constraint::Length(11),
            Constraint::Length(12),
            Constraint::Min(5),
            Constraint::Length(9),
            Constraint::Length(6),
        ])
        .split(area)
    } else {
        Layout::vertical([
            Constraint::Length(11),
            Constraint::Length(12),
            Constraint::Min(5),
            Constraint::Length(9),
        ])
        .split(area)
    };
    let recent_limit = usize::from(sections[2].height.saturating_sub(2));
    let inspector = app.task_inspector(selected, recent_limit);
    frame.render_widget(
        Paragraph::new(task_inspector_primary(&inspector))
            .block(pane_block(app, "Inspector: Task", focused))
            .wrap(Wrap { trim: false }),
        sections[0],
    );
    frame.render_widget(
        Paragraph::new(task_inspector_context(&inspector, now))
            .block(pane_block(
                app,
                "Secondary facts · Paths · Dependencies",
                false,
            ))
            .wrap(Wrap { trim: false }),
        sections[1],
    );
    yocto_logs::render(
        frame,
        sections[2],
        Text::from(task_inspector_recent_lines(app, &inspector)),
        pane_block(app, "Recent output · Recent Log (tail)", false),
        true,
    );
    let actions = task_inspector_actions(app);
    frame.render_widget(
        Paragraph::new(action_list(
            &actions,
            sections[3].width.saturating_sub(2),
            inspector_action_styles(app),
        ))
        .block(pane_block(app, "Contextual Actions", false)),
        sections[3],
    );
    if show_system {
        frame.render_widget(
            Paragraph::new(system_status_document(
                app,
                sections[4].width.saturating_sub(2),
            ))
            .block(pane_block(app, "System Status", false))
            .wrap(Wrap { trim: false }),
            sections[4],
        );
    }
}
