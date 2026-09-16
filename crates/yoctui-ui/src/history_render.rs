//! History render.
use super::*;

pub(crate) fn build_environment_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let status = match &app.build_environment {
        BuildEnvironmentState::Unconfigured => "not configured".to_owned(),
        BuildEnvironmentState::Configured(profile) => format!(
            "configured\nbuild: {}\nsource: {}\nscript: {}",
            profile.build_dir.display(),
            profile.source_dir.display(),
            profile.init_script.display()
        ),
        BuildEnvironmentState::Verifying { profile, .. } => {
            format!("verifying BitBake\nbuild: {}", profile.build_dir.display())
        }
        BuildEnvironmentState::Connected(profile) => format!(
            "connected\nbuild: {}\nsource: {}",
            profile.build_dir.display(),
            profile.source_dir.display()
        ),
        BuildEnvironmentState::Failed { profile, message } => {
            format!("failed: {message}\nbuild: {}", profile.build_dir.display())
        }
    };
    let draft = app.build_environment_draft.as_ref().map(|draft| format!(
        "\n\nEdit profile (field: {:?})\nsource: {}\nbuild: {}\nscript: {}\n\nType path, ↑/↓ select field, s save, Esc cancel.",
        draft.field, draft.source, draft.build, draft.script
    )).unwrap_or_default();
    let images = if app.build_environment.connected() && !app.available_images.is_empty() {
        format!("\n\navailable images:\n{}", app.available_images.join("\n"))
    } else {
        "\n\navailable images: locked until BitBake verification succeeds.".into()
    };
    let profile = match &app.project_profile {
        yoctui_model::ProjectProfileState::NotLoaded => "Project profile: not inspected".to_owned(),
        yoctui_model::ProjectProfileState::Absent => "Project profile: none (optional)".to_owned(),
        yoctui_model::ProjectProfileState::Invalid(message) => {
            format!("Project profile: invalid\n! {message}")
        }
        state => {
            let items =
                yoctui_model::project_profile_items(state, &app.workspace, &app.available_images);
            let rows = items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    let status = match &item.status {
                        yoctui_model::ProjectProfileItemStatus::Resolved => "resolved".into(),
                        yoctui_model::ProjectProfileItemStatus::Stale(reason) => {
                            format!("STALE: {reason}")
                        }
                        yoctui_model::ProjectProfileItemStatus::Ambiguous(count) => {
                            format!("AMBIGUOUS: {count} matches")
                        }
                        yoctui_model::ProjectProfileItemStatus::Unavailable(reason) => {
                            format!("UNAVAILABLE: {reason}")
                        }
                    };
                    format!(
                        "{} {} — {status}",
                        if index == app.project_profile_selection {
                            ">"
                        } else {
                            " "
                        },
                        item.label
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!("Project profile: team intent\n{rows}")
        }
    };
    let text = format!(
        "Build environment\n\nEnter/e Configure paths  |  b Browse directories\nA Advanced TOML  |  c Clone Poky  |  V Initialize and verify\n\n{status}{draft}{images}\n\n{profile}\n\nN/n profile item | p preview/open."
    );
    frame.render_widget(
        Paragraph::new(text)
            .block(
                Block::default()
                    .title("Build environment")
                    .borders(Borders::ALL)
                    .style(ThemePalette::for_app(app).base()),
            )
            .wrap(Wrap { trim: true }),
        area,
    );
}

pub(crate) fn job_history_detail(row: JobHistoryRowRef<'_>, now: SystemTime) -> String {
    match row {
        JobHistoryRowRef::Daemon(job) => format!(
            "Operation: {}\nType: {}  Status: {}\nAuthority: daemon\nProgress: {}\nStarted: unavailable  Finished: unavailable\nElapsed: --  Exit: {}",
            job.label,
            daemon_job_kind_label(job.kind),
            daemon_job_status_label(job.lifecycle),
            match (job.progress_current, job.progress_total) {
                (Some(current), Some(total)) if total > 0 => format!("{current}/{total}"),
                (Some(current), _) => current.to_string(),
                _ => "unavailable".into(),
            },
            job.exit_code
                .map_or_else(|| "--".into(), |code| code.to_string()),
        ),
        JobHistoryRowRef::Background(job) => {
            let outcome = job.result.as_ref().map_or_else(
                || {
                    job.error.as_ref().map_or_else(
                        || "pending".into(),
                        |error| {
                            error.detail.as_ref().map_or_else(
                                || error.summary.clone(),
                                |detail| format!("{} — {detail}", error.summary),
                            )
                        },
                    )
                },
                |result| result.summary.clone(),
            );
            let recent = job
                .output
                .back()
                .map_or("none", |entry| entry.message.as_str());
            format!(
                "Operation: {}\nType: {}  Status: {}\nContext: {}\nQueued: {}  Started: {}  Finished: {}\nElapsed: {}  Warnings: {}  Errors: {}\nOutcome: {}\nRecent output: {}",
                job.title,
                job_kind_label(job.kind),
                job_status_label(job.status),
                job_context(job),
                clock_text(job.queued_at),
                job.started_at.map_or_else(|| "--".into(), clock_text),
                job.finished_at.map_or_else(|| "--".into(), clock_text),
                job_elapsed(row, now).map_or_else(|| "--".into(), format_duration),
                job.warnings,
                job.errors,
                outcome,
                recent,
            )
        }
        JobHistoryRowRef::Build(record) => format!(
            "Operation: retained build record\nType: Build  Status: {}\nContext: {}\nStarted: unavailable  Finished: unavailable\nElapsed: {}  Exit: {}\nWarnings: {}  Errors: {}\nCompleted package tasks: {}",
            if record.success {
                "✓ Succeeded"
            } else {
                "✕ Failed"
            },
            record.target.as_deref().unwrap_or("unknown"),
            record.elapsed.map_or_else(|| "--".into(), format_duration),
            record
                .exit_code
                .map_or_else(|| "--".into(), |code| code.to_string()),
            record.warnings,
            record.errors,
            record.completed_tasks,
        ),
    }
}

pub(crate) fn build_history(frame: &mut Frame, app: &App, area: Rect, now: SystemTime) {
    if app.is_offline() || app.saved_builds.browsing {
        return crate::saved_builds::saved_build_history(frame, app, area, now);
    }
    let history = app.job_history_rows();
    let selected_index = app
        .build_history_selection
        .min(history.len().saturating_sub(1));
    let selected = history.get(selected_index).copied();
    let chunks = Layout::vertical([Constraint::Min(4), Constraint::Length(11)]).split(area);
    let capacity = usize::from(chunks[0].height.saturating_sub(3)).max(1);
    let active = history
        .iter()
        .take_while(|row| job_history_row_active(**row))
        .count();
    let pinned = active.min(capacity);
    let remaining = capacity.saturating_sub(pinned);
    let terminal_start = if selected_index < pinned || remaining == 0 {
        pinned
    } else {
        selected_index
            .saturating_add(1)
            .saturating_sub(remaining)
            .max(pinned)
            .min(history.len().saturating_sub(remaining))
    };
    let indices = (0..pinned)
        .chain(terminal_start..history.len())
        .take(capacity)
        .collect::<Vec<_>>();
    let columns = job_history_columns(chunks[0].width);
    let rows = indices.into_iter().map(|index| {
        let row = history[index];
        let selected = index == selected_index;
        let active = job_history_row_active(row);
        let style = if selected {
            selected_style(app, true)
        } else if let JobHistoryRowRef::Daemon(job) = row {
            if active {
                let palette = ThemePalette::for_app(app);
                status_tone_style(&palette, daemon_lifecycle_tone(job.lifecycle))
            } else {
                Style::default()
            }
        } else if let JobHistoryRowRef::Background(job) = row {
            if active {
                background_job_style(app, job.status)
            } else {
                Style::default()
            }
        } else {
            Style::default()
        };
        Row::new(
            columns
                .iter()
                .map(|column| job_history_cell(app, row, *column, now)),
        )
        .style(style)
    });
    frame.render_widget(
        Table::new(rows, columns.iter().map(|column| column.constraint()))
            .header(
                Row::new(columns.iter().map(|column| column.header()))
                    .style(Style::default().add_modifier(Modifier::BOLD)),
            )
            .block(
                Block::default()
                    .title(format!(
                        "Job History / Build history · {} · {} retained · {} jobs pinned",
                        job_summary_label(app, chunks[0].width),
                        history.len(),
                        active
                    ))
                    .borders(Borders::ALL),
            ),
        chunks[0],
    );
    let detail = selected.map_or_else(
        || "No background jobs or completed builds are retained in this session.".into(),
        |row| job_history_detail(row, now),
    );
    frame.render_widget(
        Paragraph::new(detail)
            .block(
                Block::default()
                    .title("Selected job detail")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        chunks[1],
    );
}
