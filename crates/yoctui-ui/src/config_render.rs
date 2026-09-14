//! Config render.
use super::*;

pub(crate) fn config_variables(app: &App) -> Vec<(&String, &String)> {
    let mut variables = app.workspace.variables.iter().collect::<Vec<_>>();
    variables.sort_by_key(|(name, _)| *name);
    variables.retain(|(name, value)| {
        matches_metadata(&app.metadata_query, &[name.as_str(), value.as_str()])
    });
    variables
}

pub(crate) fn config_inspector(app: &App) -> String {
    let variables = config_variables(app);
    let Some((name, summary_value)) = variables.get(app.config_selection).copied() else {
        let state = if app.workspace.variables.is_empty() {
            "No configuration variables supplied by the backend."
        } else {
            "No configuration variables match the active search."
        };
        return format!("{state}\n\n{}", config_copy_status(app));
    };
    let identity = VariableIdentity {
        name: name.clone(),
        recipe: app.config_scope.clone(),
    };
    if app.variable_detail_loading.contains(&identity) {
        return format!(
            "Variable: {name}\nEffective summary: {summary_value}\n\nLoading authoritative detail…\n\n{}",
            config_copy_status(app)
        );
    }
    if let Some(error) = app.variable_detail_errors.get(&identity) {
        return format!(
            "Variable: {name}\nEffective summary: {summary_value}\n\nDetail unavailable: {error}\nPress Enter to retry.\n\n{}",
            config_copy_status(app)
        );
    }
    let Some(detail) = app.variable_details.get(&identity) else {
        return format!(
            "Variable: {name}\nEffective summary: {summary_value}\nScope: {}\n\nDetail not loaded; press Enter to inspect.\n\n{}",
            app.config_scope.as_deref().unwrap_or("global"),
            config_copy_status(app)
        );
    };
    let operations = if detail.operations.is_empty() {
        "none reported".into()
    } else {
        detail
            .operations
            .iter()
            .map(|operation| {
                let source = operation.file.as_ref().map_or_else(
                    || "source unavailable".into(),
                    |file| {
                        operation.line.map_or_else(
                            || file.display().to_string(),
                            |line| format!("{}:{line}", file.display()),
                        )
                    },
                );
                format!(
                    "{} @ {}{}",
                    operation.operation,
                    source,
                    operation
                        .value
                        .as_ref()
                        .map_or_else(String::new, |value| format!(" = {value}"))
                )
            })
            .collect::<Vec<_>>()
            .join("\n  ")
    };
    format!(
        "Variable: {}\nScope: {}\nEffective value: {}\nUnexpanded value: {}\nProvenance: {}\nActive overrides: {}\nOperations:\n  {}\n{}",
        detail.identity.name,
        detail
            .identity
            .recipe
            .as_deref()
            .map_or("global", |recipe| recipe),
        detail.effective_value.as_deref().unwrap_or("unavailable"),
        detail.unexpanded_value.as_deref().unwrap_or("unavailable"),
        detail.provenance.as_deref().unwrap_or("unavailable"),
        if detail.active_overrides.is_empty() {
            "none reported".into()
        } else {
            detail.active_overrides.join(", ")
        },
        operations,
        config_copy_status(app),
    )
}

pub(crate) fn config_copy_status(app: &App) -> String {
    let mut copy_disabled_reasons = Vec::new();
    let copy = [
        ("C effective", ConfigCopyValue::Effective),
        ("U unexpanded", ConfigCopyValue::Unexpanded),
    ]
    .into_iter()
    .map(|(label, value)| {
        selected_config_copy_value(app, value).map_or_else(
            |reason| {
                copy_disabled_reasons.push(format!("{label}: {reason}"));
                format!("{label}: disabled")
            },
            |_| format!("{label}: enabled"),
        )
    })
    .collect::<Vec<_>>()
    .join(" | ");
    let source = config_source_disabled_reason(app).map_or_else(
        || "o source: enabled".into(),
        |reason| format!("o source: disabled ({reason})"),
    );
    let scope = if app.workspace.recipes.is_empty() {
        "s scope: global only (no recipes reported)".into()
    } else {
        format!(
            "s scope: enabled ({} recipes; active {})",
            app.workspace.recipes.len(),
            app.config_scope.as_deref().unwrap_or("global")
        )
    };
    let compare = config_comparison(app).map_or_else(
        |reason| format!("c compare: disabled ({reason})"),
        |_| "c compare: enabled".into(),
    );
    let edit = if config_edit_disabled_reason(app).is_none() {
        "E edit: enabled"
    } else {
        "E edit: disabled"
    };
    let copy_disabled_reasons = if copy_disabled_reasons.is_empty() {
        String::new()
    } else {
        format!("\n{}", copy_disabled_reasons.join("\n"))
    };
    format!("{copy} | {edit}\n{scope}\n{compare}\n{source}{copy_disabled_reasons}")
}

pub(crate) fn config(frame: &mut Frame, app: &App, area: Rect) {
    let variables = config_variables(app);
    let variable_count = variables.len();
    let chunks = Layout::vertical([Constraint::Percentage(32), Constraint::Min(5)]).split(area);
    let list = Layout::vertical([Constraint::Length(1), Constraint::Min(3)]).split(chunks[0]);
    frame.render_widget(
        Paragraph::new(search_line(
            app,
            &app.metadata_query,
            app.metadata_searching,
            (variable_count > 0).then_some(app.config_selection),
            variable_count,
            SearchNavigation::Results,
            SearchExit::Done,
            list[0].width,
        )),
        list[0],
    );
    let capacity = usize::from(list[1].height.saturating_sub(3)).max(1);
    let viewport = yoctui_model::centered_viewport_range(
        (variable_count > 0).then_some(app.config_selection),
        variable_count,
        capacity,
    );
    frame.render_widget(
        Table::new(
            variables[viewport.clone()]
                .iter()
                .enumerate()
                .map(|(index, (name, value))| {
                    let index = viewport.start + index;
                    Row::new(vec![Cell::from(name.as_str()), Cell::from(value.as_str())])
                        .style(selected_style(app, index == app.config_selection))
                }),
            [Constraint::Percentage(35), Constraint::Percentage(65)],
        )
        .header(
            Row::new(["Variable", "Effective value"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(format!(
                    "Effective configuration (shown: {} of {}, read-only)",
                    variable_count,
                    app.workspace.variables.len()
                ))
                .borders(Borders::ALL),
        ),
        list[1],
    );
    let detail = config_inspector(app);
    frame.render_widget(
        Paragraph::new(format!(
            "{detail}\n\nEnter refreshes detail; o opens provenance when available."
        ))
        .block(
            Block::default()
                .title("Selected variable")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: false }),
        chunks[1],
    );
}
pub(crate) fn bbmask(frame: &mut Frame, app: &App, area: Rect) {
    let value = app.workspace.variables.get("BBMASK").map_or(
        "(BBMASK is not set in the effective configuration)",
        String::as_str,
    );
    let provenance = app
        .workspace
        .variable_provenance
        .get("BBMASK")
        .map_or("backend did not provide source provenance", String::as_str);
    let patterns = value
        .split_whitespace()
        .enumerate()
        .map(|(index, pattern)| format!("{:>3}. {pattern}", index + 1))
        .collect::<Vec<_>>();
    let pattern_text = if patterns.is_empty() {
        "No masked recipe patterns are active.".into()
    } else {
        patterns.join("\n")
    };
    frame.render_widget(
        Paragraph::new(format!(
            "Effective BBMASK patterns:\n{pattern_text}\n\nProvenance: {provenance}\n\ne edits the value; Yoctui previews the exact local.conf assignment and requires confirmation."
        ))
        .block(Block::default().title("Effective BBMASK").borders(Borders::ALL))
        .wrap(Wrap { trim: false }),
        area,
    );
}
