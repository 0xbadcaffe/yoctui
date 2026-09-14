//! Job history.
use super::*;

pub(crate) fn background_job_style(app: &App, status: BackgroundJobStatus) -> Style {
    let palette = ThemePalette::for_app(app);
    match status {
        BackgroundJobStatus::Succeeded => palette.role(palette.success, Modifier::BOLD),
        BackgroundJobStatus::Failed | BackgroundJobStatus::Lost => {
            palette.role(palette.error, Modifier::BOLD)
        }
        BackgroundJobStatus::Cancelled | BackgroundJobStatus::Cancelling => {
            palette.role(palette.warning, Modifier::BOLD)
        }
        BackgroundJobStatus::Queued => palette.role(palette.pending, Modifier::BOLD),
        BackgroundJobStatus::Starting | BackgroundJobStatus::Running => {
            palette.role(palette.running, Modifier::BOLD)
        }
    }
}

pub(crate) fn job_status_label(status: BackgroundJobStatus) -> &'static str {
    match status {
        BackgroundJobStatus::Queued => "· Queued",
        BackgroundJobStatus::Starting => "… Starting",
        BackgroundJobStatus::Running => "▶ Running",
        BackgroundJobStatus::Cancelling => "! Cancelling",
        BackgroundJobStatus::Succeeded => "✓ Succeeded",
        BackgroundJobStatus::Failed => "✕ Failed",
        BackgroundJobStatus::Cancelled => "■ Cancelled",
        BackgroundJobStatus::Lost => "? Lost",
    }
}

pub(crate) fn job_kind_label(kind: BackgroundJobKind) -> &'static str {
    match kind {
        BackgroundJobKind::Build => "Build",
        BackgroundJobKind::CveCheck => "CVE check",
        BackgroundJobKind::Spdx => "SPDX",
        BackgroundJobKind::Qemu => "QEMU",
        BackgroundJobKind::Wic => "Wic",
        BackgroundJobKind::Sdk => "SDK",
        BackgroundJobKind::Test => "Test",
        BackgroundJobKind::Devtool => "Devtool",
        BackgroundJobKind::Maintenance => "Maintenance",
    }
}

pub(crate) fn daemon_job_status_label(
    lifecycle: yoctui_model::ClientDaemonLifecycle,
) -> &'static str {
    match lifecycle {
        yoctui_model::ClientDaemonLifecycle::Disconnected => "– Disconnected",
        yoctui_model::ClientDaemonLifecycle::Connecting => "… Starting",
        yoctui_model::ClientDaemonLifecycle::Running => "▶ Running",
        yoctui_model::ClientDaemonLifecycle::Stopping => "! Cancelling",
        yoctui_model::ClientDaemonLifecycle::Exited => "✓ Succeeded",
        yoctui_model::ClientDaemonLifecycle::Failed => "✕ Failed",
        yoctui_model::ClientDaemonLifecycle::Lost => "? Lost",
    }
}

pub(crate) fn daemon_job_kind_label(kind: yoctui_model::ClientDaemonJobKind) -> &'static str {
    match kind {
        yoctui_model::ClientDaemonJobKind::BitBakeBuild => "Build",
        yoctui_model::ClientDaemonJobKind::Devtool => "Devtool",
        yoctui_model::ClientDaemonJobKind::Qemu => "QEMU",
        yoctui_model::ClientDaemonJobKind::Wic => "Wic",
        yoctui_model::ClientDaemonJobKind::Sdk => "SDK",
        yoctui_model::ClientDaemonJobKind::Testing => "Test",
        yoctui_model::ClientDaemonJobKind::Qa => "QA",
        yoctui_model::ClientDaemonJobKind::Security => "Security",
        yoctui_model::ClientDaemonJobKind::Maintenance => "Maintenance",
        yoctui_model::ClientDaemonJobKind::Utility => "Utility",
        yoctui_model::ClientDaemonJobKind::Raw => "Raw",
        yoctui_model::ClientDaemonJobKind::Unknown => "Unknown",
    }
}

pub(crate) fn job_history_row_active(row: JobHistoryRowRef<'_>) -> bool {
    match row {
        JobHistoryRowRef::Daemon(job) => !job.lifecycle.is_terminal(),
        JobHistoryRowRef::Background(job) => !job.status.is_terminal(),
        JobHistoryRowRef::Build(_) => false,
    }
}

pub(crate) fn job_context(job: &yoctui_model::BackgroundJob) -> String {
    let mut values = Vec::new();
    if let Some(target) = job.context.target.as_ref() {
        values.push(target.clone());
    }
    match (job.context.recipe.as_ref(), job.context.task.as_ref()) {
        (Some(recipe), Some(task)) => values.push(format!("{recipe}:{task}")),
        (Some(recipe), None) => values.push(recipe.clone()),
        (None, Some(task)) => values.push(task.clone()),
        (None, None) => {}
    }
    if let Some(image) = job.context.image.as_ref() {
        values.push(image.clone());
    }
    if let Some(path) = job.context.path.as_ref() {
        values.push(path.display().to_string());
    }
    if values.is_empty() {
        "unavailable".into()
    } else {
        values.join(" · ")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum JobHistoryColumn {
    Id,
    Status,
    Operation,
    Type,
    Context,
    Started,
    Finished,
    Elapsed,
}

impl JobHistoryColumn {
    pub(crate) fn header(self) -> &'static str {
        match self {
            Self::Id => "ID",
            Self::Status => "Status",
            Self::Operation => "Operation",
            Self::Type => "Type",
            Self::Context => "Target / Context",
            Self::Started => "Started",
            Self::Finished => "Finished",
            Self::Elapsed => "Elapsed",
        }
    }

    pub(crate) fn constraint(self) -> Constraint {
        match self {
            Self::Id => Constraint::Length(6),
            Self::Status => Constraint::Length(13),
            Self::Operation => Constraint::Percentage(24),
            Self::Type => Constraint::Length(12),
            Self::Context => Constraint::Percentage(28),
            Self::Started | Self::Finished => Constraint::Length(9),
            Self::Elapsed => Constraint::Min(9),
        }
    }
}

pub(crate) fn job_history_columns(width: u16) -> Vec<JobHistoryColumn> {
    let mut columns = vec![
        JobHistoryColumn::Status,
        JobHistoryColumn::Operation,
        JobHistoryColumn::Context,
        JobHistoryColumn::Elapsed,
    ];
    if width >= 84 {
        columns.insert(2, JobHistoryColumn::Type);
        columns.insert(columns.len() - 1, JobHistoryColumn::Started);
    }
    if width >= 118 {
        columns.insert(0, JobHistoryColumn::Id);
        columns.insert(columns.len() - 1, JobHistoryColumn::Finished);
    }
    columns
}

pub(crate) fn job_summary_label(app: &App, width: u16) -> String {
    let summary = app.job_summary();
    let daemon = summary.daemon_owned.map_or_else(String::new, |count| {
        if width >= 84 {
            format!(" · Daemon-owned {count}")
        } else {
            format!(" D{count}")
        }
    });
    if width >= 84 {
        format!(
            "Active {} · Queued {} · Failed {} · Recent complete {}{}",
            summary.active, summary.queued, summary.failed, summary.recent_completed, daemon
        )
    } else {
        format!(
            "A{} Q{} F{} Done{}{}",
            summary.active, summary.queued, summary.failed, summary.recent_completed, daemon
        )
    }
}

pub(crate) fn job_elapsed(row: JobHistoryRowRef<'_>, now: SystemTime) -> Option<Duration> {
    match row {
        JobHistoryRowRef::Daemon(_) => None,
        JobHistoryRowRef::Background(job) => job
            .started_at
            .and_then(|started| job.finished_at.unwrap_or(now).duration_since(started).ok()),
        JobHistoryRowRef::Build(record) => record.elapsed,
    }
}

pub(crate) fn job_history_cell(
    app: &App,
    row: JobHistoryRowRef<'_>,
    column: JobHistoryColumn,
    now: SystemTime,
) -> Cell<'static> {
    match (row, column) {
        (JobHistoryRowRef::Daemon(job), JobHistoryColumn::Id) => Cell::from(job.id.to_string()),
        (JobHistoryRowRef::Background(job), JobHistoryColumn::Id) => {
            Cell::from(job.id.0.to_string())
        }
        (JobHistoryRowRef::Build(_), JobHistoryColumn::Id) => Cell::from("--"),
        (JobHistoryRowRef::Daemon(job), JobHistoryColumn::Status) => {
            let palette = ThemePalette::for_app(app);
            Cell::from(Span::styled(
                daemon_job_status_label(job.lifecycle),
                status_tone_style(&palette, daemon_lifecycle_tone(job.lifecycle)),
            ))
        }
        (JobHistoryRowRef::Background(job), JobHistoryColumn::Status) => Cell::from(Span::styled(
            job_status_label(job.status),
            background_job_style(app, job.status),
        )),
        (JobHistoryRowRef::Build(record), JobHistoryColumn::Status) => {
            let palette = ThemePalette::for_app(app);
            let (label, style) = if record.success {
                ("✓ Succeeded", palette.role(palette.success, Modifier::BOLD))
            } else {
                ("✕ Failed", palette.role(palette.error, Modifier::BOLD))
            };
            Cell::from(Span::styled(label, style))
        }
        (JobHistoryRowRef::Background(job), JobHistoryColumn::Operation) => {
            Cell::from(job.title.clone())
        }
        (JobHistoryRowRef::Daemon(job), JobHistoryColumn::Operation) => {
            Cell::from(job.label.clone())
        }
        (JobHistoryRowRef::Build(_), JobHistoryColumn::Operation) => Cell::from("Build record"),
        (JobHistoryRowRef::Daemon(job), JobHistoryColumn::Type) => {
            Cell::from(daemon_job_kind_label(job.kind))
        }
        (JobHistoryRowRef::Background(job), JobHistoryColumn::Type) => {
            Cell::from(job_kind_label(job.kind))
        }
        (JobHistoryRowRef::Build(_), JobHistoryColumn::Type) => Cell::from("Build"),
        (JobHistoryRowRef::Daemon(job), JobHistoryColumn::Context) => {
            let progress = match (job.progress_current, job.progress_total) {
                (Some(current), Some(total)) if total > 0 => format!("{current}/{total}"),
                (Some(current), _) => current.to_string(),
                _ => "daemon".into(),
            };
            Cell::from(progress)
        }
        (JobHistoryRowRef::Background(job), JobHistoryColumn::Context) => {
            Cell::from(job_context(job))
        }
        (JobHistoryRowRef::Build(record), JobHistoryColumn::Context) => {
            Cell::from(record.target.clone().unwrap_or_else(|| "unknown".into()))
        }
        (JobHistoryRowRef::Daemon(_), JobHistoryColumn::Started)
        | (JobHistoryRowRef::Daemon(_), JobHistoryColumn::Finished) => Cell::from("--"),
        (JobHistoryRowRef::Background(job), JobHistoryColumn::Started) => {
            Cell::from(job.started_at.map_or_else(|| "--".into(), clock_text))
        }
        (JobHistoryRowRef::Build(_), JobHistoryColumn::Started) => Cell::from("--"),
        (JobHistoryRowRef::Background(job), JobHistoryColumn::Finished) => {
            Cell::from(job.finished_at.map_or_else(|| "--".into(), clock_text))
        }
        (JobHistoryRowRef::Build(_), JobHistoryColumn::Finished) => Cell::from("--"),
        (row, JobHistoryColumn::Elapsed) => {
            Cell::from(job_elapsed(row, now).map_or_else(|| "--".into(), format_duration))
        }
    }
}

pub(crate) fn render_job_history(frame: &mut Frame, app: &App, area: Rect, now: SystemTime) {
    let history = app.job_history_rows();
    let title = format!("Job History · {}", job_summary_label(app, area.width));
    let columns = job_history_columns(area.width);
    let capacity = usize::from(area.height.saturating_sub(3));
    let rows = history.iter().copied().take(capacity).map(|row| {
        let active = job_history_row_active(row);
        let style = match row {
            JobHistoryRowRef::Daemon(job) if active => {
                let palette = ThemePalette::for_app(app);
                status_tone_style(&palette, daemon_lifecycle_tone(job.lifecycle))
            }
            JobHistoryRowRef::Background(job) if active => background_job_style(app, job.status),
            _ => Style::default(),
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
                Row::new(columns.iter().map(|column| column.header())).style({
                    let palette = ThemePalette::for_app(app);
                    palette.role(palette.table_header, Modifier::BOLD)
                }),
            )
            .block(Block::default().title(title).borders(Borders::ALL)),
        area,
    );
}

pub(crate) fn tasks_workspace(
    frame: &mut Frame,
    app: &App,
    area: Rect,
    now: SystemTime,
    rows: &[TaskRowRef<'_>],
) {
    let selected = rows.get(app.task_progress_scroll);
    let heights = yoctui_app::task_workspace_panel_heights(app, area.width, area.height);
    let panels = Layout::vertical(heights.map(Constraint::Length)).split(area);
    render_task_table(frame, app, panels[0], rows, now);
    if heights[1] > 0 {
        render_task_log(frame, app, panels[1], selected);
    }
    if heights[2] > 0 {
        render_job_history(frame, app, panels[2], now);
    }
    if heights[3] == 8 {
        render_telemetry_strip(frame, app, panels[3]);
    } else {
        render_compact_telemetry_strip(frame, app, panels[3]);
    }
}
