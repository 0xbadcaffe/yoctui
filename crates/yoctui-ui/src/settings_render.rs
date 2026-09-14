//! Settings render.
use super::*;

pub(crate) fn settings_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let rows = app.preference_rows();
    let control_height = if app.preferences.density == yoctui_model::UiDensity::Compact {
        4
    } else {
        5
    };
    let chunks =
        Layout::vertical([Constraint::Min(8), Constraint::Length(control_height)]).split(area);
    let visible = usize::from(chunks[0].height.saturating_sub(3)).max(1);
    let range =
        yoctui_model::centered_viewport_range(Some(app.settings_selection), rows.len(), visible);
    frame.render_widget(
        Table::new(
            rows.iter()
                .enumerate()
                .skip(range.start)
                .take(visible)
                .map(|(index, row)| {
                    Row::new([
                        row.label.to_owned(),
                        row.value.clone(),
                        if row.enabled() { "editable" } else { "locked" }.into(),
                    ])
                    .style(selected_style(app, index == app.settings_selection))
                }),
            [
                Constraint::Percentage(42),
                Constraint::Percentage(40),
                Constraint::Percentage(18),
            ],
        )
        .header(
            Row::new(["Setting", "Active value", "State"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(if app.settings_dirty {
                    "Settings (not saved)"
                } else {
                    "Settings"
                })
                .borders(Borders::ALL),
        ),
        chunks[0],
    );
    let selected_detail = rows
        .get(app.settings_selection)
        .and_then(|row| row.disabled_reason)
        .unwrap_or("Changes preview immediately and are saved atomically for the next launch.");
    frame.render_widget(
        Paragraph::new(format!(
            "↑/↓ or j/k select  ←/→ or Enter change/open  R reset all  r retry\n{selected_detail}\nBuild actions stay disabled until the environment connection is verified."
        ))
        .block(
            Block::default()
                .title("Settings controls")
                .borders(Borders::ALL)
                .style(ThemePalette::for_app(app).base()),
        )
        .wrap(Wrap { trim: true }),
        chunks[1],
    );
}

pub(crate) fn keymap_preferences_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let state = &app.keymap_preferences_ui;
    let rows = yoctui_model::keymap_preference_rows(
        &app.keymap_preferences,
        &app.effective_keymap,
        &state.query,
    );
    let width = area.width.saturating_sub(4).clamp(76, 116);
    let height = area.height.saturating_sub(2).clamp(22, 38);
    let popup = Rect::new(
        (area.width.saturating_sub(width)) / 2,
        (area.height.saturating_sub(height)) / 2,
        width,
        height,
    );
    clear_popup(frame, app, popup);
    let chunks = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(8),
        Constraint::Length(5),
    ])
    .split(popup);
    let palette = ThemePalette::for_app(app);
    let query = if state.searching {
        format!("Search: {}_", state.query)
    } else if state.query.is_empty() {
        "Search: <all> · / edit · Ctrl+U clear".into()
    } else {
        format!("Search: {} · / edit · Ctrl+U clear", state.query)
    };
    frame.render_widget(
        Paragraph::new(query)
            .style(palette.base())
            .block(dialog_block(
                app,
                format!(
                    "Keybinding preferences · {} result{}",
                    rows.len(),
                    if rows.len() == 1 { "" } else { "s" }
                ),
                DialogTone::Standard,
            )),
        chunks[0],
    );

    let visible = usize::from(chunks[1].height.saturating_sub(3)).max(1);
    let selected = state.selection.min(rows.len().saturating_sub(1));
    let start = selected
        .saturating_sub(visible / 2)
        .min(rows.len().saturating_sub(visible));
    let rendered = rows
        .iter()
        .enumerate()
        .skip(start)
        .take(visible)
        .map(|(index, row)| {
            let marker = if index == selected { "▶" } else { " " };
            Row::new([
                format!("{marker} {}", row.label),
                row.scope.to_string(),
                row.binding_label(),
                format!(
                    "{}{}",
                    row.state_label(),
                    if row.critical { " · critical" } else { "" }
                ),
            ])
            .style(selected_style(app, index == selected))
        });
    frame.render_widget(
        Table::new(
            rendered,
            [
                Constraint::Percentage(30),
                Constraint::Percentage(23),
                Constraint::Percentage(27),
                Constraint::Percentage(20),
            ],
        )
        .header(
            Row::new(["Action", "Scope", "Effective binding", "State"])
                .style(palette.role(palette.accent, Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title("Effective keymap")
                .borders(Borders::ALL),
        ),
        chunks[1],
    );

    let selected_row = rows.get(selected);
    let identity = selected_row.map_or_else(
        || "No binding matches the current search.".into(),
        |row| format!("{} · {}", row.action_id.as_str(), row.menu_path.join(" > ")),
    );
    let (status, status_style) = if let Some(error) = state.validation_error.as_ref() {
        (
            format!("Conflict/disabled: {error}"),
            palette.role(palette.error, Modifier::BOLD),
        )
    } else if let Some(capture) = state.capture.as_ref() {
        (
            format!(
                "Pending capture: {} · Ctrl+S validate/save · Backspace · Esc cancel",
                capture.label()
            ),
            palette.role(palette.warning, Modifier::BOLD),
        )
    } else {
        (
            "Enter/c capture · x remove · r reset · R reset all · e export · p retry save · Esc close"
                .into(),
            palette.role(palette.secondary_foreground, Modifier::DIM),
        )
    };
    frame.render_widget(
        Paragraph::new(vec![
            Line::raw(identity),
            Line::styled(status, status_style),
        ])
        .style(palette.base())
        .block(
            Block::default()
                .title("Selected binding")
                .borders(Borders::ALL),
        )
        .wrap(Wrap { trim: true }),
        chunks[2],
    );
}

pub(crate) fn onboarding_overlay(frame: &mut Frame, app: &App, area: Rect) {
    let projection = app.onboarding_projection();
    let popup = dialog_popup_rect(
        area,
        if area.width >= 120 { 112 } else { 76 },
        if area.height >= 34 { 32 } else { 22 },
    );
    clear_popup(frame, app, popup);
    let palette = ThemePalette::for_app(app);
    let regions = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(8),
        Constraint::Length(if popup.height >= 28 { 8 } else { 6 }),
        Constraint::Length(1),
    ])
    .split(popup);

    let completed = projection
        .rows
        .iter()
        .filter(|row| row.status == yoctui_model::OnboardingStepStatus::Completed)
        .count();
    frame.render_widget(
        Paragraph::new(format!(
            "{completed}/{} completed · opening/resuming starts no build or process",
            projection.rows.len()
        ))
        .style(palette.base())
        .block(dialog_block(
            app,
            "Yoctui workflow guide · focus trapped",
            DialogTone::Standard,
        ))
        .wrap(Wrap { trim: true }),
        regions[0],
    );

    let rows = projection.rows.iter().map(|row| {
        let marker = match row.status {
            yoctui_model::OnboardingStepStatus::Completed => "[x]",
            yoctui_model::OnboardingStepStatus::Current => "[>]",
            yoctui_model::OnboardingStepStatus::Blocked => "[-]",
            yoctui_model::OnboardingStepStatus::Skipped => "[~]",
            yoctui_model::OnboardingStepStatus::Stale => "[!]",
            yoctui_model::OnboardingStepStatus::Unavailable => "[?]",
        };
        let selected = row.step == projection.selected;
        Row::new([
            format!("{marker} {}", row.title),
            row.status.label().to_owned(),
            row.destination.to_owned(),
        ])
        .style(if selected {
            selected_style(app, true)
        } else {
            match row.status {
                yoctui_model::OnboardingStepStatus::Completed => {
                    palette.role(palette.success, Modifier::BOLD)
                }
                yoctui_model::OnboardingStepStatus::Current => {
                    palette.role(palette.accent, Modifier::BOLD)
                }
                yoctui_model::OnboardingStepStatus::Stale
                | yoctui_model::OnboardingStepStatus::Unavailable => {
                    palette.role(palette.warning, Modifier::BOLD)
                }
                yoctui_model::OnboardingStepStatus::Blocked
                | yoctui_model::OnboardingStepStatus::Skipped => {
                    palette.role(palette.disabled, Modifier::DIM)
                }
            }
        })
    });
    frame.render_widget(
        Table::new(
            rows,
            [
                Constraint::Percentage(43),
                Constraint::Length(12),
                Constraint::Percentage(40),
            ],
        )
        .header(
            Row::new(["Workflow step", "State", "Authoritative destination"])
                .style(palette.role(palette.heading, Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title("Six-step workbench path")
                .borders(Borders::ALL),
        ),
        regions[1],
    );

    let selected = projection
        .rows
        .iter()
        .find(|row| row.step == projection.selected);
    let detail = selected.map_or_else(
        || "No workflow step is selected.".into(),
        |row| {
            format!(
                "{}\nPrerequisite: {}\nEnter opens {}. It does not bypass confirmation or create a process.",
                row.instruction, row.prerequisite, row.destination
            )
        },
    );
    frame.render_widget(
        Paragraph::new(detail)
            .style(palette.base())
            .block(
                Block::default()
                    .title("Selected step")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: true }),
        regions[2],
    );
    frame.render_widget(
        Paragraph::new(bounded_cell_text(
            "Esc dismiss · Enter open · n next · s skip · r restart · j/k select",
            regions[3].width,
        ))
        .style(palette.role(palette.secondary_foreground, Modifier::DIM)),
        regions[3],
    );
}
