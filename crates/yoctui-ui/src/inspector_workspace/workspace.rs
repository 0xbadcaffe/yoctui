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
