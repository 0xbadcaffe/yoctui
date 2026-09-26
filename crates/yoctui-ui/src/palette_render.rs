//! Palette render.
use super::*;

pub(crate) fn command_palette_rect(area: Rect) -> Rect {
    let (horizontal_inset, vertical_inset, maximum_width, maximum_height): (u16, u16, u16, u16) =
        match area.width {
            130.. => (6, 2, 112, 30),
            100..=129 => (3, 2, area.width, 28),
            _ => (1, 1, area.width, area.height),
        };
    let width = area
        .width
        .saturating_sub(horizontal_inset.saturating_mul(2))
        .min(maximum_width);
    let height = area
        .height
        .saturating_sub(vertical_inset.saturating_mul(2))
        .min(maximum_height);
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

pub(crate) fn palette_command_availability(
    app: &App,
    command: &yoctui_model::PaletteCommand,
) -> (String, Style) {
    let palette = ThemePalette::for_app(app);
    if !command.enabled() {
        return (
            "– Unavailable".into(),
            palette.role(palette.disabled, Modifier::DIM),
        );
    }
    match command.compatibility_state {
        WorkspaceAvailabilityState::Available => (
            "✓ Ready".into(),
            palette.role(palette.success, Modifier::BOLD),
        ),
        WorkspaceAvailabilityState::AvailableWithLimitations => (
            "! Limited".into(),
            palette.role(palette.warning, Modifier::BOLD),
        ),
        WorkspaceAvailabilityState::Unavailable => (
            "✕ Unavailable".into(),
            palette.role(palette.error, Modifier::BOLD),
        ),
        WorkspaceAvailabilityState::Unsupported => (
            "– Unsupported".into(),
            palette.role(palette.disabled, Modifier::DIM),
        ),
        WorkspaceAvailabilityState::Unknown => (
            "? Unknown".into(),
            palette.role(palette.informational, Modifier::ITALIC),
        ),
    }
}

pub(crate) fn command_palette_detail_lines(
    app: &App,
    command: Option<&yoctui_model::PaletteCommand>,
    width: u16,
    maximum: usize,
) -> Vec<Line<'static>> {
    let palette = ThemePalette::for_app(app);
    let Some(command) = command else {
        return vec![Line::styled(
            "No command selected.",
            palette.role(palette.muted, Modifier::DIM),
        )];
    };
    let mut values = vec![
        (
            command.description.to_owned(),
            palette.role(palette.informational, Modifier::ITALIC),
        ),
        (
            format!(
                "Available: {} · Compatibility: {}",
                if command.enabled() { "yes" } else { "no" },
                compatibility_workspace_state_label(command.compatibility_state)
            ),
            compatibility_workspace_state_style(app, command.compatibility_state),
        ),
    ];
    if let Some(reason) = command.disabled_reason.as_deref() {
        values.push((
            format!("Cannot run: {reason}"),
            palette.role(palette.warning, Modifier::BOLD),
        ));
    }
    if let Some(reason) = command.compatibility_reason.as_deref()
        && command.disabled_reason.as_deref() != Some(reason)
    {
        values.push((
            format!("Reason: {reason}"),
            palette.role(palette.warning, Modifier::BOLD),
        ));
    }
    if !command.compatibility_limitations.is_empty() {
        values.push((
            format!(
                "Limitations: {}",
                command.compatibility_limitations.join("; ")
            ),
            palette.role(palette.warning, Modifier::ITALIC),
        ));
    }
    if !command.implementations.is_empty() {
        values.push((
            format!(
                "Implementation: {}",
                command
                    .implementations
                    .iter()
                    .map(|(_, implementation)| implementation.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            palette.role(palette.secondary_foreground, Modifier::DIM),
        ));
    }
    values.push((
        format!(
            "Action: {} · Menu: {} · Safety: {}",
            command.action_id.as_str(),
            command.menu_path.join(" > "),
            command.safety.label()
        ),
        palette.role(palette.secondary_foreground, Modifier::DIM),
    ));
    values
        .into_iter()
        .take(maximum)
        .map(|(value, style)| Line::styled(bounded_cell_text(&value, width), style))
        .collect()
}

pub(crate) fn global_search_hit_detail_lines(
    app: &App,
    hit: &yoctui_model::GlobalSearchHit,
    width: u16,
    maximum: usize,
) -> Vec<Line<'static>> {
    let palette = ThemePalette::for_app(app);
    let mut values = vec![
        (
            format!(
                "{} · line {} · column {}",
                hit.kind.label(),
                hit.line,
                hit.column
            ),
            palette.role(palette.heading, Modifier::BOLD),
        ),
        (
            hit.path.display().to_string(),
            palette.role(palette.secondary_foreground, Modifier::DIM),
        ),
    ];
    if let Some(image) = hit.image.as_deref() {
        values.push((
            format!("Generated image: {image}"),
            palette.role(palette.informational, Modifier::BOLD),
        ));
    }
    values.push((hit.preview.clone(), palette.base()));
    values
        .into_iter()
        .take(maximum)
        .map(|(value, style)| Line::styled(bounded_cell_text(&value, width), style))
        .collect()
}

pub(crate) fn command_palette(frame: &mut Frame, app: &App, area: Rect) {
    let popup = command_palette_rect(area);
    if popup.width < 4 || popup.height < 4 {
        return;
    }
    let palette = ThemePalette::for_app(app);
    let global_search = app.command_palette_mode == CommandPaletteMode::GlobalRegexSearch;
    let workspace_search = global_search && app.global_search_root.is_some();
    clear_popup(frame, app, popup);
    let outer = Block::default()
        .title(if workspace_search {
            "Workspace Regex Search"
        } else if global_search {
            "Global Regex Search"
        } else {
            "Command Palette"
        })
        .borders(Borders::ALL)
        .style(palette.base())
        .border_style(palette.focus());
    let inner = outer.inner(popup);
    frame.render_widget(outer, popup);
    if inner.is_empty() {
        return;
    }

    let detail_height = if popup.height >= 24 { 7 } else { 5 };
    let regions = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(4),
        Constraint::Length(detail_height),
        Constraint::Length(1),
    ])
    .split(inner);
    let commands = app.filtered_command_palette_commands();
    let content_hits = if global_search {
        app.global_search_content.hits()
    } else {
        &[]
    };
    let total = commands.len() + content_hits.len();
    let regex_error = app.command_palette_regex_error();
    let selection = app.command_palette_selection.min(total.saturating_sub(1));
    frame.render_widget(
        Paragraph::new(search_line(
            app,
            &app.command_palette_query,
            true,
            (total > 0).then_some(selection),
            total,
            SearchNavigation::Results,
            SearchExit::Close,
            regions[0].width,
        )),
        regions[0],
    );

    let visible_count = usize::from(regions[1].height.saturating_sub(3)).max(1);
    let start = selection / visible_count * visible_count;
    let end = start.saturating_add(visible_count).min(total);
    let result_label = if global_search { "Results" } else { "Commands" };
    let content_state = match &app.global_search_content {
        yoctui_model::GlobalSearchContentState::Loading { .. } if global_search => " · scanning…",
        yoctui_model::GlobalSearchContentState::Ready {
            truncated: true, ..
        } if global_search => " · limit reached",
        yoctui_model::GlobalSearchContentState::Failed { .. } if global_search => " · scan failed",
        _ => "",
    };
    let viewport_cue = BoundedScrollIndicator::new(start, end.saturating_sub(start), total)
        .title_label(
            (total > 0).then_some(selection),
            true,
            app.preferences.symbols == SymbolPreference::Unicode,
        );
    let match_label = if total == 1 { "match" } else { "matches" };
    let list_title = viewport_cue.map_or_else(
        || format!("{result_label} · {total} {match_label}{content_state}"),
        |cue| format!("{result_label} · {cue}{content_state}"),
    );
    let list_block = Block::default()
        .title(list_title)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.inactive_border));
    if let Some(error) = regex_error.as_deref() {
        frame.render_widget(
            StateView {
                kind: StateKind::Error,
                summary: "Invalid regular expression.".into(),
                detail: Some(bounded_cell_text(error, regions[1].width.saturating_sub(4))),
                action: Some("Edit the expression with Backspace or clear it with Ctrl+U.".into()),
            }
            .paragraph(
                palette.role(palette.error, Modifier::BOLD),
                palette.role(palette.secondary_foreground, Modifier::DIM),
            )
            .block(list_block),
            regions[1],
        );
    } else if total == 0 && app.global_search_content.loading() {
        let activity = if app.preferences.symbols == SymbolPreference::Unicode {
            if app.reduced_motion {
                "⣿"
            } else {
                startup_activity_symbol(app.animation_frame as usize)
            }
        } else {
            "*"
        };
        frame.render_widget(
            StateView {
                kind: StateKind::Loading,
                summary: format!(
                    "{activity} Searching {} text files…",
                    if workspace_search {
                        "workspace"
                    } else {
                        "build and generated rootfs"
                    }
                ),
                detail: Some("Results are bounded and generated caches are excluded.".into()),
                action: Some("Keep typing to replace this search; Esc cancels it.".into()),
            }
            .paragraph(
                palette.role(palette.informational, Modifier::BOLD),
                palette.role(palette.secondary_foreground, Modifier::DIM),
            )
            .block(list_block),
            regions[1],
        );
    } else if total == 0
        && let yoctui_model::GlobalSearchContentState::Failed { message, .. } =
            &app.global_search_content
    {
        frame.render_widget(
            StateView {
                kind: StateKind::Error,
                summary: "Content search failed.".into(),
                detail: Some(message.clone()),
                action: Some("Edit the query to retry.".into()),
            }
            .paragraph(
                palette.role(palette.error, Modifier::BOLD),
                palette.role(palette.secondary_foreground, Modifier::DIM),
            )
            .block(list_block),
            regions[1],
        );
    } else if total == 0 {
        frame.render_widget(
            StateView {
                kind: StateKind::Empty,
                summary: if global_search && app.command_palette_query.trim().is_empty() {
                    format!(
                        "Type a regular expression to search {} file contents.",
                        if workspace_search {
                            "workspace"
                        } else {
                            "build"
                        }
                    )
                } else if global_search {
                    format!(
                        "No {} file contents match this regular expression.",
                        if workspace_search {
                            "workspace"
                        } else {
                            "build"
                        }
                    )
                } else {
                    "No commands match this search.".into()
                },
                detail: Some("Backspace edits; Ctrl+U clears the query.".into()),
                action: None,
            }
            .paragraph(
                palette.role(palette.muted, Modifier::BOLD),
                palette.role(palette.secondary_foreground, Modifier::DIM),
            )
            .block(list_block),
            regions[1],
        );
    } else if regions[1].width >= 88 {
        let rows = (0..total).skip(start).take(visible_count).map(|index| {
            let row_style = if index == selection {
                selected_style(app, true)
            } else {
                palette.base()
            };
            if let Some(command) = commands.get(index) {
                let (availability, state_style) = palette_command_availability(app, command);
                let label_style = if command.enabled() {
                    palette.base()
                } else {
                    palette.role(palette.disabled, Modifier::DIM)
                };
                Row::new([
                    Cell::from(format!(
                        "{} {}",
                        if index == selection { "▶" } else { " " },
                        command.label
                    ))
                    .style(label_style),
                    Cell::from(command.shortcut.to_owned())
                        .style(palette.role(palette.secondary_foreground, Modifier::DIM)),
                    Cell::from(availability).style(state_style),
                ])
                .style(row_style)
            } else {
                let hit = &content_hits[index - commands.len()];
                Row::new([
                    Cell::from(format!(
                        "{} {}",
                        if index == selection { "▶" } else { " " },
                        bounded_cell_text(&hit.preview, 38)
                    )),
                    Cell::from(format!("{}:{}", hit.path.display(), hit.line))
                        .style(palette.role(palette.secondary_foreground, Modifier::DIM)),
                    Cell::from(hit.kind.label())
                        .style(palette.role(palette.informational, Modifier::BOLD)),
                ])
                .style(row_style)
            }
        });
        frame.render_widget(
            Table::new(
                rows,
                [
                    Constraint::Percentage(35),
                    Constraint::Percentage(43),
                    Constraint::Percentage(22),
                ],
            )
            .header(
                Row::new(["Result", "Location / Shortcut", "Kind / Availability"])
                    .style(palette.role(palette.heading, Modifier::BOLD)),
            )
            .block(list_block)
            .column_spacing(1),
            regions[1],
        );
    } else {
        let row_width = regions[1].width.saturating_sub(2);
        let rows = (0..total).skip(start).take(visible_count).map(|index| {
            if let Some(command) = commands.get(index) {
                let (availability, state_style) = palette_command_availability(app, command);
                let value = format!(
                    "{} {} · {} · {availability}",
                    if index == selection { "▶" } else { " " },
                    command.label,
                    command.shortcut,
                );
                Row::new([bounded_cell_text(&value, row_width)]).style(if index == selection {
                    selected_style(app, true)
                } else {
                    state_style
                })
            } else {
                let hit = &content_hits[index - commands.len()];
                let value = format!(
                    "{} {} · {}:{}",
                    if index == selection { "▶" } else { " " },
                    hit.kind.label(),
                    hit.path.display(),
                    hit.line
                );
                Row::new([bounded_cell_text(&value, row_width)]).style(if index == selection {
                    selected_style(app, true)
                } else {
                    palette.base()
                })
            }
        });
        frame.render_widget(
            Table::new(rows, [Constraint::Min(1)]).block(list_block),
            regions[1],
        );
    }

    let detail_block = Block::default()
        .title(if global_search {
            "Selected result"
        } else {
            "Selected command"
        })
        .borders(Borders::ALL)
        .border_style(Style::default().fg(palette.inactive_border));
    let detail_inner = detail_block.inner(regions[2]);
    let detail_lines = if let Some(command) = commands.get(selection) {
        command_palette_detail_lines(
            app,
            Some(command),
            detail_inner.width,
            usize::from(detail_inner.height),
        )
    } else if let Some(hit) = selection
        .checked_sub(commands.len())
        .and_then(|index| content_hits.get(index))
    {
        global_search_hit_detail_lines(
            app,
            hit,
            detail_inner.width,
            usize::from(detail_inner.height),
        )
    } else {
        command_palette_detail_lines(
            app,
            None,
            detail_inner.width,
            usize::from(detail_inner.height),
        )
    };
    frame.render_widget(
        Paragraph::new(detail_lines)
            .block(detail_block)
            .wrap(Wrap { trim: false }),
        regions[2],
    );
    frame.render_widget(
        Paragraph::new(bounded_cell_text(
            if global_search {
                "Content regex · ↑/↓ select · PgUp/PgDn page · Enter open · Esc close"
            } else {
                "Esc close · Enter run · ↑/↓ select · Type search · Backspace edit · Ctrl+U clear"
            },
            regions[3].width,
        ))
        .style(palette.role(palette.secondary_foreground, Modifier::DIM)),
        regions[3],
    );
}
