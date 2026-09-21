fn render_project_dialogs(frame: &mut Frame, app: &App, area: Rect) -> bool {
    if matches!(app.active_dialog(), Some(Dialog::BuildCompletion)) {
        build_completion_popup(frame, app, area);
        return true;
    } else if matches!(
        app.active_dialog(),
        Some(Dialog::BuildCancellationConfirmation)
    ) {
        let popup = bounded_dialog_rect(area, 58, 5);
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(
                "Are you sure you want to cancel the build?\n\n[y/Enter] Cancel build  [n/Esc] Keep building",
            )
            .block(dialog_block(
                app,
                "Confirm build cancellation",
                DialogTone::Destructive,
            )),
            popup,
        );
        return true;
    } else if matches!(app.active_dialog(), Some(Dialog::QuitConfirmation)) {
        let popup = bounded_dialog_rect(area, 56, 5);
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(
                "Are you sure you want to exit yoctui?\n\n[y/Enter] Exit yoctui  [n/Esc] Stay",
            )
            .block(dialog_block(app, "Confirm exit", DialogTone::Destructive)),
            popup,
        );
        return true;
    } else if let Some(Dialog::SignatureTaskPicker(picker)) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 4,
            area.height / 4,
            area.width / 2,
            area.height / 2,
        );
        clear_popup(frame, app, popup);
        let viewport = selected_table_viewport(picker.selection, picker.tasks.len(), popup);
        frame.render_widget(
            Table::new(
                picker.tasks[viewport.clone()]
                    .iter()
                    .enumerate()
                    .map(|(offset, task)| {
                        let index = viewport.start + offset;
                        Row::new([format!(
                            "{} {task}",
                            if index == picker.selection {
                                "▶"
                            } else {
                                " "
                            }
                        )])
                        .style(selected_style(app, index == picker.selection))
                    }),
                [Constraint::Min(1)],
            )
            .header(Row::new(["Authoritative signature tasks"]).style(Style::default().bold()))
            .block(dialog_block(
                app,
                format!("Inspect signatures: {}", picker.recipe.name),
                DialogTone::Standard,
            )),
            popup,
        );
        return true;
    } else if let Some(Dialog::RecipeTaskPicker(picker)) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 4,
            area.height / 4,
            area.width / 2,
            area.height / 2,
        );
        clear_popup(frame, app, popup);
        let viewport = selected_table_viewport(picker.selection, picker.tasks.len(), popup);
        frame.render_widget(
            Table::new(
                picker.tasks[viewport.clone()]
                    .iter()
                    .enumerate()
                    .map(|(offset, task)| {
                        let index = viewport.start + offset;
                        Row::new([format!(
                            "{} {task}",
                            if index == picker.selection {
                                "▶"
                            } else {
                                " "
                            }
                        )])
                        .style(selected_style(app, index == picker.selection))
                    }),
                [Constraint::Min(1)],
            )
            .header(Row::new(["Authoritative BitBake tasks"]).style(Style::default().bold()))
            .block(dialog_block(
                app,
                format!(
                    "{} task: {}",
                    if picker.force { "Force" } else { "Run" },
                    picker.recipe
                ),
                DialogTone::Standard,
            )),
            popup,
        );
        return true;
    } else if let Some(Dialog::RecipeTaskLogPicker(picker)) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 5,
            area.height / 4,
            area.width * 3 / 5,
            area.height / 2,
        );
        clear_popup(frame, app, popup);
        let viewport = selected_table_viewport(picker.selection, picker.logs.len(), popup);
        frame.render_widget(
            Table::new(
                picker.logs[viewport.clone()]
                    .iter()
                    .enumerate()
                    .map(|(offset, log)| {
                        let index = viewport.start + offset;
                        Row::new([
                            Cell::from(format!(
                                "{} {}",
                                if index == picker.selection {
                                    "▶"
                                } else {
                                    " "
                                },
                                log.task
                            )),
                            Cell::from(format!("{:?}", log.state)),
                            Cell::from(log.path.display().to_string()),
                        ])
                        .style(selected_style(app, index == picker.selection))
                    }),
                [
                    Constraint::Length(20),
                    Constraint::Length(12),
                    Constraint::Min(20),
                ],
            )
            .header(
                Row::new(["Task", "State", "Authoritative log path"])
                    .style(Style::default().bold()),
            )
            .block(dialog_block(
                app,
                format!("{} retained task logs", picker.recipe),
                DialogTone::Standard,
            )),
            popup,
        );
        return true;
    } else if let Some(Dialog::ConfigEdit {
        identity: _,
        editor,
    }) = app.active_dialog()
    {
        toml_popup_editor(frame, app, area, "Configuration.toml", editor, None);
        return true;
    } else if let Some(Dialog::ConfigEditConfirmation(request)) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 8,
            area.height / 4,
            area.width * 3 / 4,
            area.height / 2,
        );
        clear_popup(frame, app, popup);
        frame.render_widget(
            Paragraph::new(format!(
                "Destination:\n{}\n\nExact assignment:\n{}\n\nEnter confirms; Esc cancels.",
                request.destination.display(),
                request.assignment
            ))
            .block(dialog_block(
                app,
                "Preview configuration edit",
                DialogTone::Confirmation,
            ))
            .wrap(Wrap { trim: false }),
            popup,
        );
        return true;
    } else if let Some(Dialog::ConfigComparison(comparison)) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 8,
            area.height / 5,
            area.width * 3 / 4,
            area.height * 3 / 5,
        );
        clear_popup(frame, app, popup);
        let field = |name: &str, value: &yoctui_model::ConfigComparisonField| {
            format!(
                "{name}: {:?}\n  global: {}\n  {}: {}",
                value.outcome,
                value.global.as_deref().unwrap_or("unavailable"),
                comparison.recipe,
                value.recipe.as_deref().unwrap_or("unavailable")
            )
        };
        frame.render_widget(
            Paragraph::new(format!(
                "Variable: {}\nGlobal vs recipe {}\n\n{}\n\n{}\n\nEnter or Esc closes.",
                comparison.variable,
                comparison.recipe,
                field("Effective", &comparison.effective),
                field("Unexpanded", &comparison.unexpanded),
            ))
            .block(dialog_block(
                app,
                "Configuration comparison",
                DialogTone::Result,
            ))
            .wrap(Wrap { trim: false }),
            popup,
        );
        return true;
    } else if let Some(Dialog::ConfigScopePicker(picker)) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 5,
            area.height / 4,
            area.width * 3 / 5,
            area.height / 2,
        );
        clear_popup(frame, app, popup);
        let viewport = selected_table_viewport(picker.selection, picker.scopes.len(), popup);
        frame.render_widget(
            Table::new(
                picker.scopes[viewport.clone()]
                    .iter()
                    .enumerate()
                    .map(|(offset, scope)| {
                        let index = viewport.start + offset;
                        Row::new([format!(
                            "{} {}",
                            if index == picker.selection {
                                "▶"
                            } else {
                                " "
                            },
                            scope.as_deref().unwrap_or("(global)")
                        )])
                        .style(selected_style(app, index == picker.selection))
                    }),
                [Constraint::Min(20)],
            )
            .header(Row::new(["Variable scope"]).style(Style::default().bold()))
            .block(dialog_block(
                app,
                format!("{} scope", picker.variable),
                DialogTone::Standard,
            )),
            popup,
        );
        return true;
    } else if let Some(Dialog::ConfigSourcePicker(picker)) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 8,
            area.height / 4,
            area.width * 3 / 4,
            area.height / 2,
        );
        clear_popup(frame, app, popup);
        let viewport = selected_table_viewport(picker.selection, picker.sources.len(), popup);
        frame.render_widget(
            Table::new(
                picker.sources[viewport.clone()]
                    .iter()
                    .enumerate()
                    .map(|(offset, source)| {
                        let index = viewport.start + offset;
                        Row::new([
                            Cell::from(format!(
                                "{} {}",
                                if index == picker.selection {
                                    "▶"
                                } else {
                                    " "
                                },
                                source.operation
                            )),
                            Cell::from(source.path.display().to_string()),
                            Cell::from(
                                source
                                    .line
                                    .map_or_else(|| "—".into(), |line| line.to_string()),
                            ),
                        ])
                        .style(selected_style(app, index == picker.selection))
                    }),
                [
                    Constraint::Length(12),
                    Constraint::Min(20),
                    Constraint::Length(7),
                ],
            )
            .header(
                Row::new(["Operation", "Authoritative source", "Line"])
                    .style(Style::default().bold()),
            )
            .block(dialog_block(
                app,
                format!("{} defining sources", picker.identity.name),
                DialogTone::Standard,
            )),
            popup,
        );
        return true;
    } else if let Some(Dialog::RecipePatchPicker(picker)) = app.active_dialog() {
        let popup = Rect::new(
            area.width / 5,
            area.height / 4,
            area.width * 3 / 5,
            area.height / 2,
        );
        clear_popup(frame, app, popup);
        let viewport = selected_table_viewport(picker.selection, picker.patches.len(), popup);
        frame.render_widget(
            Table::new(
                picker.patches[viewport.clone()]
                    .iter()
                    .enumerate()
                    .map(|(offset, patch)| {
                        let index = viewport.start + offset;
                        Row::new([format!(
                            "{} {}",
                            if index == picker.selection {
                                "▶"
                            } else {
                                " "
                            },
                            patch.display()
                        )])
                        .style(selected_style(app, index == picker.selection))
                    }),
                [Constraint::Min(20)],
            )
            .header(Row::new(["Authoritative local patch"]).style(Style::default().bold()))
            .block(dialog_block(
                app,
                format!("{} patch review", picker.recipe),
                DialogTone::Standard,
            )),
            popup,
        );
        return true;
    }
    false
}
