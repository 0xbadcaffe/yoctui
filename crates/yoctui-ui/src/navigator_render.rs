//! Navigator render.
use super::*;

pub(crate) fn navigator(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    task_rows: Option<&[TaskRowRef<'_>]>,
) {
    if area.width == 26 && app.screen == Screen::Tasks {
        literal_project_navigator(frame, app, area, task_rows.unwrap_or_default());
        return;
    }
    const DESTINATIONS: [(&str, Screen, WorkspaceDestination); 25] = [
        (
            "Dashboard",
            Screen::Dashboard,
            WorkspaceDestination::Dashboard,
        ),
        (
            "Insights",
            Screen::Insights,
            WorkspaceDestination::Dashboard,
        ),
        ("Layers", Screen::Layers, WorkspaceDestination::Layers),
        ("Recipes", Screen::Recipes, WorkspaceDestination::Recipes),
        ("Packages", Screen::Packages, WorkspaceDestination::Packages),
        ("Images", Screen::Images, WorkspaceDestination::Images),
        ("Kernel", Screen::Kernel, WorkspaceDestination::Kernel),
        (
            "U-Boot / BIOS",
            Screen::Firmware,
            WorkspaceDestination::Firmware,
        ),
        ("SDK", Screen::Sdk, WorkspaceDestination::Sdk),
        ("Tasks", Screen::Tasks, WorkspaceDestination::Tasks),
        ("Logs", Screen::Logs, WorkspaceDestination::Logs),
        ("Errors", Screen::Errors, WorkspaceDestination::Errors),
        (
            "Configuration",
            Screen::Configuration,
            WorkspaceDestination::Configuration,
        ),
        (
            "Dependencies",
            Screen::Dependencies,
            WorkspaceDestination::Dependencies,
        ),
        ("Testing", Screen::Testing, WorkspaceDestination::Testing),
        ("Security", Screen::Security, WorkspaceDestination::Security),
        ("QA", Screen::Qa, WorkspaceDestination::Qa),
        ("Raw Mode", Screen::RawMode, WorkspaceDestination::RawMode),
        (
            "Terminal Sessions",
            Screen::TerminalSessions,
            WorkspaceDestination::TerminalSessions,
        ),
        ("Devtool", Screen::Recipes, WorkspaceDestination::Devtool),
        ("QEMU / Wic", Screen::Images, WorkspaceDestination::QemuWic),
        (
            "Maintenance",
            Screen::Maintenance,
            WorkspaceDestination::Maintenance,
        ),
        (
            "Build environment",
            Screen::BuildEnvironment,
            WorkspaceDestination::BuildEnvironment,
        ),
        (
            "Compatibility",
            Screen::Compatibility,
            WorkspaceDestination::Compatibility,
        ),
        ("Settings", Screen::Settings, WorkspaceDestination::Settings),
    ];
    enum NavigatorRow<'a> {
        Group {
            name: &'a str,
            index: usize,
        },
        Destination {
            name: &'a str,
            destination: WorkspaceDestination,
            index: usize,
        },
    }

    let mut rows = Vec::new();
    for (group_index, group) in NAVIGATOR_GROUPS.iter().enumerate() {
        rows.push(NavigatorRow::Group {
            name: group.label,
            index: group_index,
        });
        if app.navigator_groups_expanded[group_index] {
            rows.extend(DESTINATIONS[group.start..group.end].iter().enumerate().map(
                |(offset, (name, _, destination))| NavigatorRow::Destination {
                    name,
                    destination: *destination,
                    index: group.start + offset,
                },
            ));
        }
    }

    let expected_visible = usize::from(area.height.saturating_sub(2));
    let total = app.navigator_visible_row_count();
    let start = app.navigator_viewport_start(expected_visible);
    let indicator = BoundedScrollIndicator::new(start, expected_visible, total);
    let title = indicator
        .title_label(
            Some(app.navigator_visual_row()),
            false,
            app.preferences.symbols == SymbolPreference::Unicode,
        )
        .map_or_else(|| "Navigator".into(), |cue| format!("Navigator · {cue}"));
    let block = pane_block(app, &title, app.focus == FocusTarget::Navigator);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }
    let visible = usize::from(inner.height);
    let start = app.navigator_viewport_start(visible);
    let palette = ThemePalette::for_app(app);
    let width = usize::from(inner.width);
    let lines = rows
        .iter()
        .skip(start)
        .take(visible)
        .map(|row| match row {
            NavigatorRow::Group { name, index } => {
                let expanded = app.navigator_groups_expanded[*index];
                let selected = !expanded && app.navigator_group_index() == *index;
                Line::from(Span::styled(
                    format!(
                        "{} {name:<width$}",
                        if expanded { "▾" } else { "▸" },
                        width = width.saturating_sub(2)
                    ),
                    if selected {
                        palette.selected()
                    } else {
                        palette.role(palette.warning, Modifier::BOLD)
                    },
                ))
            }
            NavigatorRow::Destination {
                name,
                destination,
                index,
            } => {
                let selected = *index == app.navigator_selection;
                let availability = compatibility_ui_workspace_destination_action_availability(
                    &app.workspace_compatibility,
                    *destination,
                );
                let marker = match availability.state {
                    WorkspaceAvailabilityState::Available => "▱",
                    WorkspaceAvailabilityState::AvailableWithLimitations => "~",
                    WorkspaceAvailabilityState::Unavailable => "×",
                    WorkspaceAvailabilityState::Unknown => " ",
                    WorkspaceAvailabilityState::Unsupported => "!",
                };
                let badge = navigator_badge(app, *destination);
                let content_width = width.saturating_sub(4);
                let badge_width = badge.chars().count();
                let label_width = content_width.saturating_sub(badge_width);
                let label = name.chars().take(label_width).collect::<String>();
                Line::from(Span::styled(
                    format!("  {marker} {label:<label_width$}{badge}"),
                    if selected {
                        palette.selected()
                    } else {
                        compatibility_workspace_state_style(app, availability.state)
                    },
                ))
            }
        })
        .collect::<Vec<_>>();
    frame.render_widget(Paragraph::new(Text::from(lines)), inner);
}

pub(crate) fn navigator_badge(app: &App, destination: WorkspaceDestination) -> String {
    match destination {
        WorkspaceDestination::Tasks => {
            let active = app
                .tasks
                .values()
                .filter(|task| task.state == TaskState::Active)
                .count();
            if active > 0 {
                format!(" {active}")
            } else {
                String::new()
            }
        }
        WorkspaceDestination::Errors => {
            if app.build.errors > 0 {
                format!(" {}", app.build.errors)
            } else {
                String::new()
            }
        }
        WorkspaceDestination::Logs if app.logs.paused_len.is_some() => " PAUSE".into(),
        WorkspaceDestination::Logs if app.logs.follow && !app.is_offline() => " LIVE".into(),
        WorkspaceDestination::Devtool => {
            if app.devtool_statuses.is_empty() {
                String::new()
            } else {
                format!(" {}", app.devtool_statuses.len())
            }
        }
        _ => String::new(),
    }
}

pub(crate) fn project_tree_child(
    label: &str,
    marker: &str,
    width: usize,
    selected: bool,
    palette: ThemePalette,
) -> Line<'static> {
    let content_width = width.saturating_sub(4);
    let label = label.chars().take(content_width).collect::<String>();
    let row = format!("  {marker} {label:<content_width$}");
    if selected {
        Line::styled(row, palette.selected())
    } else {
        Line::from(vec![
            Span::styled(
                format!("  {marker} "),
                palette.role(palette.warning, Modifier::BOLD),
            ),
            Span::raw(format!("{label:<content_width$}")),
        ])
    }
}

pub(crate) fn project_tree_group(
    label: &str,
    width: usize,
    palette: ThemePalette,
) -> Line<'static> {
    Line::styled(
        format!("▾ {label:<width$}", width = width.saturating_sub(2)),
        palette.role(palette.warning, Modifier::BOLD),
    )
}

pub(crate) fn literal_project_navigator(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    task_rows: &[TaskRowRef<'_>],
) {
    let palette = ThemePalette::for_app(app);
    let block = pane_block(app, "Navigator", app.focus == FocusTarget::Navigator);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.is_empty() {
        return;
    }
    let sections = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);
    let width = usize::from(sections[0].width);
    let selected_screen = app.navigator_screen();
    let mut lines = Vec::new();

    lines.push(project_tree_group("Layers", width, palette));
    if app.workspace.layers.is_empty() {
        lines.push(project_tree_child(
            "discovering…",
            "▱",
            width,
            selected_screen == Screen::Layers,
            palette,
        ));
    } else {
        for (index, layer) in app.workspace.layers.iter().take(7).enumerate() {
            lines.push(project_tree_child(
                &layer.name,
                "▱",
                width,
                selected_screen == Screen::Layers && index == app.layer_selection,
                palette,
            ));
        }
    }

    lines.push(project_tree_group("Recipes", width, palette));
    if app.workspace.recipes.is_empty() {
        lines.push(project_tree_child(
            "discovering…",
            "▸",
            width,
            selected_screen == Screen::Recipes,
            palette,
        ));
    } else {
        for (index, recipe) in app.workspace.recipes.iter().take(3).enumerate() {
            lines.push(project_tree_child(
                &recipe.name,
                "▸",
                width,
                selected_screen == Screen::Recipes && index == app.recipe_selection,
                palette,
            ));
        }
    }

    lines.push(project_tree_group("Images", width, palette));
    if app.available_images.is_empty() {
        lines.push(project_tree_child(
            "none discovered",
            "▱",
            width,
            selected_screen == Screen::Images,
            palette,
        ));
    } else {
        for image in app.available_images.iter().take(2) {
            lines.push(project_tree_child(
                image,
                "▱",
                width,
                selected_screen == Screen::Images
                    && app.build.target.as_deref() == Some(image.as_str()),
                palette,
            ));
        }
    }

    lines.push(project_tree_group("Tasks", width, palette));
    for (label, screen) in [
        ("Build", Screen::Tasks),
        ("Test", Screen::Testing),
        ("QA", Screen::Qa),
        ("Devtool", Screen::Recipes),
        ("Wic", Screen::Images),
        ("SDK", Screen::Sdk),
        ("Security", Screen::Security),
        ("Utility", Screen::Maintenance),
    ] {
        lines.push(project_tree_child(
            label,
            "▱",
            width,
            selected_screen == screen,
            palette,
        ));
    }

    lines.push(project_tree_group("Targets", width, palette));
    let machine = app
        .workspace
        .variables
        .get("MACHINE")
        .map_or("unknown", String::as_str);
    lines.push(project_tree_child(machine, "▱", width, false, palette));
    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: false }),
        sections[0],
    );

    let layer = app
        .workspace
        .layers
        .get(app.layer_selection)
        .map_or("--", |layer| layer.name.as_str());
    let job = app
        .daemon
        .jobs
        .last()
        .map_or_else(|| "--".into(), |job| job.id.to_string());
    let pid = task_rows
        .get(app.task_progress_scroll)
        .and_then(|row| match row {
            TaskRowRef::Task { task, .. } => task.pid,
            TaskRowRef::WaitingSummary(_) => None,
        });
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("L:", palette.role(palette.warning, Modifier::BOLD)),
            Span::raw(format!(" {layer}  ")),
            Span::styled("R:", palette.role(palette.success, Modifier::BOLD)),
            Span::raw(format!(" {job}  ")),
            Span::styled("P:", palette.role(palette.informational, Modifier::BOLD)),
            Span::raw(pid.map_or_else(|| "--".into(), |pid| pid.to_string())),
        ])),
        sections[1],
    );
}
