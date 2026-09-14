//! Raw catalog render.
use super::*;

pub(crate) fn raw_category_kind_label(kind: yoctui_model::RawCategoryKind) -> &'static str {
    match kind {
        yoctui_model::RawCategoryKind::Favorites => "FAVORITES",
        yoctui_model::RawCategoryKind::Executable => "BITBAKE",
        yoctui_model::RawCategoryKind::ReferenceOnly => "REFERENCE",
        yoctui_model::RawCategoryKind::Conceptual => "CONCEPT",
        yoctui_model::RawCategoryKind::CompanionTools => "COMPANION",
    }
}

pub(crate) fn raw_category_kind_style(app: &App, kind: yoctui_model::RawCategoryKind) -> Style {
    let palette = ThemePalette::for_app(app);
    match kind {
        yoctui_model::RawCategoryKind::Favorites => palette.role(palette.warning, Modifier::BOLD),
        yoctui_model::RawCategoryKind::Executable => palette.role(palette.success, Modifier::BOLD),
        yoctui_model::RawCategoryKind::ReferenceOnly => {
            palette.role(palette.informational, Modifier::BOLD)
        }
        yoctui_model::RawCategoryKind::Conceptual => palette.role(palette.muted, Modifier::DIM),
        yoctui_model::RawCategoryKind::CompanionTools => {
            palette.role(palette.accent, Modifier::BOLD)
        }
    }
}

pub(crate) fn raw_category_browser(frame: &mut Frame, app: &App, area: Rect) {
    let catalog = yoctui_model::builtin_raw_catalog();
    let categories = catalog.browser_categories();
    let selection = app.raw_mode.category.as_ref().and_then(|selected| {
        categories
            .iter()
            .position(|category| &category.id == selected)
    });
    let viewport_height = usize::from(area.height.saturating_sub(2));
    let viewport =
        yoctui_model::centered_viewport_range(selection, categories.len(), viewport_height);
    let position =
        BoundedScrollIndicator::new(viewport.start, viewport.len(), categories.len()).label();
    let active = app.focus == FocusTarget::Workspace
        && app.raw_mode.browser_column == yoctui_model::RawBrowserColumn::Categories;
    let title = format!("Raw Mode / Categories · {position}");
    let block = pane_block(app, &title, active);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }

    let shell = PaneShell::new("", active, pane_styles(app));
    let width = usize::from(inner.width);
    let viewport_start = viewport.start;
    let lines = categories[viewport]
        .iter()
        .enumerate()
        .map(|(offset, category)| {
            let index = viewport_start + offset;
            let selected = selection == Some(index);
            let kind = raw_category_kind_label(category.kind);
            let prefix = format!("{} [{kind}] ", if selected { "▶" } else { " " });
            let label_width = width.saturating_sub(prefix.chars().count());
            let label = bounded_cell_text(&category.label, label_width as u16);
            Line::styled(
                format!("{prefix}{label}"),
                if selected {
                    shell.row_style(true, active)
                } else {
                    raw_category_kind_style(app, category.kind)
                },
            )
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), inner);
}

pub(crate) fn raw_availability_label(state: yoctui_model::RawAvailabilityState) -> &'static str {
    match state {
        yoctui_model::RawAvailabilityState::Available => "AVAILABLE",
        yoctui_model::RawAvailabilityState::Limited => "LIMITED",
        yoctui_model::RawAvailabilityState::Unavailable => "UNAVAILABLE",
        yoctui_model::RawAvailabilityState::Unknown => "UNKNOWN",
        yoctui_model::RawAvailabilityState::Unsupported => "UNSUPPORTED",
    }
}

pub(crate) fn raw_availability_style(
    app: &App,
    state: yoctui_model::RawAvailabilityState,
) -> Style {
    let palette = ThemePalette::for_app(app);
    match state {
        yoctui_model::RawAvailabilityState::Available => {
            palette.role(palette.success, Modifier::BOLD)
        }
        yoctui_model::RawAvailabilityState::Limited => {
            palette.role(palette.warning, Modifier::BOLD)
        }
        yoctui_model::RawAvailabilityState::Unavailable => {
            palette.role(palette.error, Modifier::BOLD)
        }
        yoctui_model::RawAvailabilityState::Unknown => palette.role(palette.muted, Modifier::DIM),
        yoctui_model::RawAvailabilityState::Unsupported => {
            palette.role(palette.disabled, Modifier::DIM)
        }
    }
}

pub(crate) fn raw_command_template(command: &yoctui_model::RawCommand) -> String {
    match &command.execution {
        yoctui_model::RawExecutionPolicy::Executable { template } => template
            .display_template(&command.parameters)
            .unwrap_or_else(|| command.reference.command.clone()),
        yoctui_model::RawExecutionPolicy::ReferenceOnly { .. } => command.reference.command.clone(),
    }
}

pub(crate) fn raw_command_list(frame: &mut Frame, app: &App, area: Rect) {
    let catalog = yoctui_model::builtin_raw_catalog();
    let commands = app.raw_mode.visible_commands(catalog);
    let selection = app
        .raw_mode
        .command
        .as_ref()
        .and_then(|selected| commands.iter().position(|command| &command.id == selected));
    let row_height = 2usize;
    let visible_rows = usize::from(area.height.saturating_sub(2)) / row_height;
    let viewport =
        yoctui_model::centered_viewport_range(selection, commands.len(), visible_rows.max(1));
    let position =
        BoundedScrollIndicator::new(viewport.start, viewport.len(), commands.len()).label();
    let active = app.focus == FocusTarget::Workspace
        && app.raw_mode.browser_column == yoctui_model::RawBrowserColumn::Commands;
    let title = format!("Raw Mode / Commands · {position}");
    let block = pane_block(app, &title, active);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }

    if commands.is_empty() {
        let selected_category = app
            .raw_mode
            .category
            .as_ref()
            .and_then(|category| catalog.category(category));
        let message = if selected_category
            .is_some_and(|category| category.kind == yoctui_model::RawCategoryKind::Favorites)
        {
            "No favorite Raw commands.\n\nf marks a selected command as a favorite.\nLeft/h returns to categories."
        } else if app.raw_mode.search.query.is_empty() {
            "No Raw commands in this category.\n\nLeft/h returns to categories."
        } else {
            "No Raw commands match this search.\n\nCtrl+U clears the query.\nLeft/h returns to categories."
        };
        frame.render_widget(Paragraph::new(message).wrap(Wrap { trim: false }), inner);
        return;
    }

    let shell = PaneShell::new("", active, pane_styles(app));
    let width = inner.width;
    let viewport_start = viewport.start;
    let authority = app.workspace_compatibility.authority();
    let lines = commands[viewport]
        .iter()
        .enumerate()
        .flat_map(|(offset, command)| {
            let index = viewport_start + offset;
            let selected = selection == Some(index);
            let availability = command.availability(authority);
            let execution = if matches!(
                command.execution,
                yoctui_model::RawExecutionPolicy::Executable { .. }
            ) {
                "RUN"
            } else {
                "REF"
            };
            let favorite = if app.raw_mode.is_favorite(&command.id) {
                "FAV"
            } else {
                "NOT-FAV"
            };
            let template = bounded_cell_text(
                &format!(
                    "{} {}",
                    if selected { "▶" } else { " " },
                    raw_command_template(command)
                ),
                width,
            );
            let metadata = bounded_cell_text(
                &format!(
                    "  {execution} · {} · {favorite}",
                    raw_availability_label(availability.state)
                ),
                width,
            );
            let selection_style = shell.row_style(true, active);
            [
                Line::styled(
                    template,
                    if selected {
                        selection_style
                    } else {
                        ThemePalette::for_app(app).base()
                    },
                ),
                Line::styled(
                    metadata,
                    if selected {
                        selection_style
                    } else {
                        raw_availability_style(app, availability.state)
                    },
                ),
            ]
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(lines), inner);
}

pub(crate) fn raw_parameter_kind_label(kind: yoctui_model::RawParameterKind) -> &'static str {
    match kind {
        yoctui_model::RawParameterKind::Recipe => "Recipe",
        yoctui_model::RawParameterKind::Image => "Image",
        yoctui_model::RawParameterKind::Target => "Target",
        yoctui_model::RawParameterKind::Task => "Task",
        yoctui_model::RawParameterKind::UserInterface => "User interface",
        yoctui_model::RawParameterKind::File => "File",
        yoctui_model::RawParameterKind::Integer => "Number",
        yoctui_model::RawParameterKind::Text => "Value",
        yoctui_model::RawParameterKind::Multiconfig => "Multiconfig",
    }
}

pub(crate) fn raw_parameter_presence_label(
    presence: yoctui_model::RawParameterPresence,
) -> &'static str {
    match presence {
        yoctui_model::RawParameterPresence::Required => "Required",
        yoctui_model::RawParameterPresence::Optional => "Optional",
    }
}

pub(crate) fn raw_reference_kind_label(kind: yoctui_model::RawReferenceKind) -> &'static str {
    match kind {
        yoctui_model::RawReferenceKind::ShellPipeline => "shell pipeline",
        yoctui_model::RawReferenceKind::Conceptual => "conceptual material",
        yoctui_model::RawReferenceKind::CompanionTool => "companion tool",
        yoctui_model::RawReferenceKind::UnsupportedBitBake => "unsupported BitBake command",
    }
}

pub(crate) fn raw_interaction_label(interaction: yoctui_model::RawInteractionMode) -> &'static str {
    match interaction {
        yoctui_model::RawInteractionMode::NoninteractiveJob => "Noninteractive job",
        yoctui_model::RawInteractionMode::InteractivePty => "Interactive PTY",
    }
}

pub(crate) fn raw_safety_label(safety: yoctui_model::RawSafetyClass) -> &'static str {
    match safety {
        yoctui_model::RawSafetyClass::Inspection => "Read only",
        yoctui_model::RawSafetyClass::Build => "Build/mutating",
        yoctui_model::RawSafetyClass::MetadataMutation => "Build/mutating (metadata mutation)",
        yoctui_model::RawSafetyClass::Destructive => "Destructive",
        yoctui_model::RawSafetyClass::ServerLifecycle => "Server lifecycle",
    }
}

pub(crate) fn raw_command_help_text(app: &App) -> String {
    let catalog = yoctui_model::builtin_raw_catalog();
    let Some(command) = app.raw_mode.selected_command(catalog) else {
        return "No Raw command selected.\n\nSelect a command in the Workspace to inspect its exact catalog help."
            .into();
    };
    let availability = command.availability(app.workspace_compatibility.authority());
    let mut lines = vec![
        format!("Description: {}", command.reference.description),
        format!("Reference section: {}", command.reference.heading),
        format!("Template: {}", raw_command_template(command)),
        format!(
            "Availability: {}",
            raw_availability_label(availability.state)
        ),
    ];

    if availability.issues.is_empty() {
        lines.push("Reason: none".into());
    } else {
        for issue in &availability.issues {
            let capability = issue
                .capability
                .map_or_else(|| "reference".into(), |capability| capability.to_string());
            lines.push(format!("Reason [{capability}]: {}", issue.reason));
            lines.extend(
                issue
                    .limitations
                    .iter()
                    .map(|limitation| format!("Limitation [{capability}]: {limitation}")),
            );
        }
    }

    if availability.implementations.is_empty() {
        lines.push("Implementation: none selected".into());
    } else {
        lines.extend(
            availability
                .implementations
                .iter()
                .map(|(capability, implementation)| {
                    format!("Implementation [{capability}]: {implementation}")
                }),
        );
    }

    match &command.execution {
        yoctui_model::RawExecutionPolicy::Executable { template } => {
            lines.push(format!(
                "Interaction: {}",
                raw_interaction_label(template.interaction)
            ));
            lines.push(format!("Safety: {}", raw_safety_label(template.safety)));
        }
        yoctui_model::RawExecutionPolicy::ReferenceOnly { kind, .. } => {
            lines.push(format!(
                "Interaction: Reference only ({})",
                raw_reference_kind_label(*kind)
            ));
            lines.push("Safety: Unsupported reference".into());
        }
    }

    if command.parameters.is_empty() {
        lines.push("Parameters: none".into());
    } else {
        lines.push("Parameters:".into());
        lines.extend(command.parameters.iter().map(|parameter| {
            format!(
                "- {} {} · {} · {}",
                parameter.label,
                parameter.placeholder,
                raw_parameter_kind_label(parameter.kind),
                raw_parameter_presence_label(parameter.presence)
            )
        }));
    }
    lines.push(format!(
        "Favorite: {}",
        if app.raw_mode.is_favorite(&command.id) {
            "Yes"
        } else {
            "No"
        }
    ));
    lines.join("\n")
}
