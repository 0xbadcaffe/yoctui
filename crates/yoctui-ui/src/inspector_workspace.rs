//! Inspector workspace.
use super::*;

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

pub(crate) fn inspector(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    now: SystemTime,
    task_rows: Option<&[TaskRowRef<'_>]>,
) {
    if app.screen == Screen::Dashboard {
        dashboard_inspector(frame, app, area, now);
        return;
    }
    if app.screen == Screen::Tasks {
        tasks_inspector(frame, app, area, now, task_rows.unwrap_or_default());
        return;
    }
    if app.screen == Screen::Errors
        && frame.area().height >= 50
        && area.height >= 32
        && app.focus != FocusTarget::Navigator
    {
        concept_error_inspector(frame, app, area);
        return;
    }
    if app.focus == FocusTarget::Navigator {
        frame.render_widget(
            Paragraph::new(compatibility_destination_detail(
                app,
                app.navigator_compatibility_destination(),
                area.width.saturating_sub(2),
            ))
            .block(pane_block(
                app,
                &format!("Inspector: {}", app.inspector_mode().label()),
                false,
            ))
            .wrap(Wrap { trim: false }),
            area,
        );
        return;
    }
    let details = match app.screen {
        Screen::Recipes => app.workspace.recipes.get(app.recipe_selection).map_or_else(
            || "No recipe selected.".into(),
            |recipe| recipe_inspector(app, recipe),
        ),
        Screen::Layers => app.layer_browser.as_ref().map_or_else(
            || {
                app.workspace.layers.get(app.layer_selection).map_or_else(
                    || "No layer selected.".into(),
                    |layer| {
                        format!(
                            "Layer: {}\nPath: {}\nPriority: {}\n\nEnter/Right opens this layer tree.",
                            layer.name,
                            layer.path.display(),
                            layer
                                .priority
                                .map_or_else(|| "unknown".into(), |value| value.to_string())
                        )
                    },
                )
            },
            |browser| {
                browser.selected_entry().map_or_else(
                    || format!("Layer: {}\n\nThis layer is empty.", browser.layer),
                    |entry| {
                        let detail = layer_entry_metadata(app, browser, entry);
                        let preview = match browser.preview_kind {
                            PreviewKind::Binary => "Binary preview unavailable.",
                            PreviewKind::Text if !browser.preview.is_empty() => {
                                "Text preview is visible in the workspace."
                            }
                            _ => "Preview unavailable.",
                        };
                        format!("{detail}\n\n{preview}")
                    },
                )
            },
        ),
        Screen::Configuration => config_inspector(app),
        Screen::Logs => match app.log_workspace_view {
            LogWorkspaceView::BitBake => app.logs.selected().map_or_else(
                || "No BitBake logs retained.".into(),
                |entry| {
                    format!(
                        "Authority: BitBake domain log\nTime: {}\nSeverity: {:?}\nBuild: {}\nRecipe: {}\nTask: {}\nSource: {}\nProtected: {}\nBookmarked: {}",
                        timestamp_text(entry.timestamp),
                        entry.severity,
                        entry.build.as_deref().unwrap_or("unavailable"),
                        entry.recipe.as_deref().unwrap_or("unavailable"),
                        entry.task.as_deref().unwrap_or("unavailable"),
                        entry.path.as_ref().map_or_else(
                            || "unavailable".into(),
                            |path| path.display().to_string()
                        ),
                        if entry.protected { "yes" } else { "no" },
                        if app.logs.is_bookmarked(entry.id) { "yes" } else { "no" },
                    )
                },
            ),
            LogWorkspaceView::Yoctui => app.internal_logs.selected().map_or_else(
                || "No Yoctui self-diagnostics retained.".into(),
                |entry| {
                    format!(
                        "Authority: local Yoctui tracing\nTime: {}\nLevel: {}\nTarget: {}\nRecord ID: {}\n\nThis entry is never interpreted as BitBake output.",
                        timestamp_text(entry.timestamp),
                        entry.level.label(),
                        entry.target,
                        entry.id
                    )
                },
            ),
        },
        Screen::Errors => app
            .logs
            .diagnostics()
            .nth(app.error_selection)
            .map_or_else(
                || "No retained warnings or errors.".into(),
                |entry| diagnostic_detail(app, entry),
            ),
        Screen::Dependencies | Screen::LayerRelationships => dependency_inspector(app),
        Screen::Signatures => signature_detail_text(app),
        Screen::BuildHistory if app.is_offline() || app.saved_builds.browsing => app.saved_builds.records.get(app.saved_builds.selection).map_or_else(
            || "No saved build selected.".into(),
            |r| format!("Saved build · read-only\nTarget: {}\nMachine: {}\nOutcome: {:?}\nSaved log lines: {}\nRecorded tasks: {}\n\nNo live process authority.\nEnter details; Left/Right changes views.",r.target,r.machine.as_deref().unwrap_or("not recorded"),r.outcome,r.logs.len(),r.tasks.len())),
        Screen::BuildHistory => app
            .job_history_rows()
            .get(app.build_history_selection)
            .copied()
            .map_or_else(
                || "No background jobs or completed builds are retained in this session.".into(),
                |row| job_history_detail(row, now),
            ),
        Screen::Packages => package_inspector_text(app),
        Screen::Images => image_artifact_inspector_text(app),
        Screen::Kernel => platform_inspector_text(&app.kernel, "Kernel"),
        Screen::Firmware => platform_inspector_text(&app.firmware, "U-Boot / BIOS"),
        Screen::Sdk => sdk_inspector_text(app),
        Screen::Testing => testing_inspector_text(app),
        Screen::Security => security_inspector_text(app),
        Screen::Qa => qa_inspector_text(app),
        Screen::RawMode => raw_command_help_text(app),
        Screen::TerminalSessions => terminal_session_inspector_text(app),
        Screen::Maintenance => maintenance_inspector_text(app),
        Screen::Compatibility => compatibility_inspector_text(app),
        _ => format!(
            "Target: {}\nStatus: {:?}\n\nSelect an item in the workspace to inspect its details.",
            app.build.target.as_deref().unwrap_or("not selected"),
            app.build.status
        ),
    };
    let destination = yoctui_model::workspace_screen_destination(app.screen);
    let actions = compatibility_workspace_actions(app, destination);
    let related_paths = inspector_related_paths(app);
    let secondary = (matches!(app.screen, Screen::Dashboard | Screen::BuildHistory)
        && !app.is_offline()
        && !(app.screen == Screen::BuildHistory && app.saved_builds.browsing))
        .then(|| job_summary_label(app, area.width));
    let recent_output = match app.screen {
        Screen::Logs => match app.log_workspace_view {
            LogWorkspaceView::BitBake => app.logs.selected().map(|entry| entry.message.as_str()),
            LogWorkspaceView::Yoctui => app
                .internal_logs
                .selected()
                .map(|entry| entry.message.as_str()),
        },
        Screen::BuildHistory if app.is_offline() || app.saved_builds.browsing => None,
        Screen::BuildHistory => app
            .job_history_rows()
            .get(app.build_history_selection)
            .and_then(|row| match row {
                JobHistoryRowRef::Daemon(_) => None,
                JobHistoryRowRef::Background(job) => {
                    job.output.back().map(|entry| entry.message.as_str())
                }
                JobHistoryRowRef::Build(_) => None,
            }),
        _ => None,
    };
    let status = (app.screen == Screen::Dashboard)
        .then(|| system_status_text(app, area.width.saturating_sub(2)));
    let show_actions = !(app.screen == Screen::Dashboard && area.height < 30);
    let document = inspector_document(
        app,
        InspectorDocumentSections {
            primary: &details,
            secondary: secondary.as_deref(),
            related_paths: &related_paths,
            recent_output,
            actions: (show_actions
                && !actions.is_empty()
                && !(app.screen == Screen::BuildHistory
                    && (app.is_offline() || app.saved_builds.browsing)))
                .then_some(actions.as_slice()),
            status: status.as_deref(),
        },
        area.width.saturating_sub(2),
    );
    let title = format!("Inspector: {}", app.inspector_mode().label());
    let block = pane_block(app, &title, app.focus == FocusTarget::Inspector);
    if matches!(
        app.screen,
        Screen::Errors
            | Screen::Recipes
            | Screen::BuildHistory
            | Screen::Images
            | Screen::Sdk
            | Screen::Testing
            | Screen::Security
            | Screen::Qa
            | Screen::Maintenance
    ) || (app.screen == Screen::Logs && app.log_workspace_view == LogWorkspaceView::BitBake)
    {
        yocto_logs::render(frame, area, document, block, true);
    } else {
        frame.render_widget(
            Paragraph::new(document)
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        );
    }
}

#[allow(dead_code)]
pub(crate) fn dashboard_inspector(frame: &mut Frame, app: &App, area: Rect, now: SystemTime) {
    if app.is_offline() {
        frame.render_widget(Paragraph::new(format!("Offline workspace\n\n[E] Configure build environment\n[F3] Saved build history\n\nSaved builds: {}\n\nStart or reconnect the daemon for live build operations.\n\nLocal files and saved records remain available. Previous observations do not establish current build state.", app.saved_builds.records.len())).wrap(Wrap { trim: true }).block(Block::default().borders(Borders::ALL).title("Project Inspector")), area);
        return;
    }
    let palette = ThemePalette::for_app(app);
    let center = app.command_center_projection_at(now);
    let dashboard = &center.dashboard;
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
    let project = app.build.target.as_deref().unwrap_or("not selected");
    let source = app
        .workspace
        .source_dir
        .as_deref()
        .map_or_else(|| "unavailable".into(), |path| path.display().to_string());
    let build_dir = app
        .workspace
        .build_dir
        .as_deref()
        .map_or_else(|| "unavailable".into(), |path| path.display().to_string());
    let disk = app.host_telemetry.disk_available_bytes.map_or_else(
        || "unavailable".into(),
        |bytes| format!("{} free", format_bytes(bytes)),
    );
    let artifact = dashboard.artifacts.first().map_or_else(
        || "Artifact: none retained".into(),
        |artifact| {
            let name = artifact.path.file_name().map_or_else(
                || artifact.path.display().to_string(),
                |name| name.to_string_lossy().into_owned(),
            );
            format!("Artifact: {} · {name}", artifact.source.label())
        },
    );
    let recent = dashboard.recent_work.first().copied().map_or_else(
        || "Recent: none retained".into(),
        |row| format!("Recent: {}", dashboard_recent_work_line(row, now)),
    );
    let context = center.recent_contexts.first().copied().map_or_else(
        || "Context: none retained".into(),
        |row| format!("Context: {}", command_center_context_line(row)),
    );
    let favorite = center.favorite_commands.first().map_or_else(
        || "Favorite: none".into(),
        |favorite| format!("Favorite: {}", favorite.favorite.name),
    );
    let terminal = center.terminals.first().map_or_else(
        || "Terminal: none".into(),
        |terminal| {
            format!(
                "Terminal: {} · {}",
                terminal.name,
                daemon_lifecycle_label(terminal.lifecycle)
            )
        },
    );
    let attention = dashboard.failures.first().map_or_else(
        || {
            if app.build.warnings > 0 || app.build.errors > 0 {
                format!(
                    "Build reports {} warning(s), {} error(s); no diagnostic rows. [e] opens diagnostics.",
                    app.build.warnings, app.build.errors
                )
            } else {
                "No retained warnings or errors".into()
            }
        },
        |entry| {
            let message = entry.message.lines().next().unwrap_or("diagnostic");
            let line_width = area.width.saturating_sub(4);
            message.find(" failed").map_or_else(
                || bounded_status_line(message.to_owned(), line_width),
                |split| {
                    if split <= usize::from(line_width) {
                        format!(
                            "{}\n{}",
                            bounded_status_line(message[..split].to_owned(), line_width),
                            bounded_status_line(message[split + 1..].to_owned(), line_width)
                        )
                    } else {
                        bounded_status_line(message.to_owned(), line_width)
                    }
                },
            )
        },
    );
    let mut lines = vec![
        Line::styled(
            "Environment",
            palette.role(palette.informational, Modifier::BOLD),
        ),
        Line::from(format!("Project          : {project}")),
        Line::from(format!("Machine          : {machine}")),
        Line::from(format!("Distro           : {distro}")),
        Line::from(format!(
            "Yocto release    : {}",
            app.workspace.release.as_deref().unwrap_or("unknown")
        )),
        Line::from(format!("Layers enabled   : {}", app.workspace.layers.len())),
        Line::default(),
        Line::styled(
            "Workspace",
            palette.role(palette.informational, Modifier::BOLD),
        ),
        Line::from(format!("Source           : {source}")),
        Line::from(format!("Build directory  : {build_dir}")),
        Line::from(format!("Disk             : {disk}")),
        Line::default(),
        Line::styled(
            "Enabled Actions",
            palette.role(palette.informational, Modifier::BOLD),
        ),
        Line::from("[B] Build image"),
        Line::from("[F2] Monitor tasks"),
        Line::from("[l] View logs"),
        Line::from("[e] View errors"),
        Line::from("[E] Verify environment"),
        Line::from("[t] Open terminal"),
        Line::from("[d] Open devtool"),
        Line::default(),
        Line::styled(
            "Active Tasks",
            palette.role(palette.informational, Modifier::BOLD),
        ),
        Line::from(if app.tasks.is_empty() {
            "No active task events".into()
        } else {
            format!("{} active task(s)", app.tasks.len())
        }),
        Line::styled(
            "Next Action",
            palette.role(palette.informational, Modifier::BOLD),
        ),
        Line::from(format!(
            "{} [{}]",
            dashboard.next_action.label, dashboard.next_action.shortcut
        )),
        Line::styled(
            "Attention",
            palette.role(palette.informational, Modifier::BOLD),
        ),
    ];
    lines.extend(attention.lines().map(|line| Line::from(line.to_owned())));
    lines.extend([
        Line::styled(
            "Workbench Center",
            palette.role(palette.informational, Modifier::BOLD),
        ),
        Line::from(context),
        Line::from(format!(
            "Active: tasks {} · jobs {}",
            dashboard.summary.active,
            center.active_jobs.len()
        )),
        Line::from(favorite),
        Line::from(terminal),
        Line::from("[t] Terminal sessions"),
        Line::from(artifact),
        Line::from(recent),
    ]);
    if lines.len() > usize::from(area.height.saturating_sub(2)) {
        lines.truncate(usize::from(area.height.saturating_sub(2)));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(pane_block(
                app,
                "Project Inspector",
                app.focus == FocusTarget::Inspector,
            ))
            .wrap(Wrap { trim: false }),
        area,
    );
}

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
