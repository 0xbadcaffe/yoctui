//! Typed, bounded command and content-search detail projections.
use super::*;

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
