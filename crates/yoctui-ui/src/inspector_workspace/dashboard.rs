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
