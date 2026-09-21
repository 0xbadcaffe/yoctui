impl App {
    pub fn progress_hierarchy_at(&self, now: SystemTime) -> ProgressHierarchy {
        let completed = u64::try_from(self.build.completed).unwrap_or(u64::MAX);
        let total = self.build.total.and_then(|value| u64::try_from(value).ok());
        let build = if let Some(terminal) = build_terminal(self.build.status) {
            terminal_progress("Build", completed, total, terminal, "final")
        } else if let Some(total) = total {
            GaugeProjection::determinate("Build", completed, total, WidgetRole::Progress)
        } else if matches!(
            self.build.status,
            BuildStatus::LoadingWorkspace | BuildStatus::Parsing | BuildStatus::Running
        ) {
            GaugeProjection::indeterminate("Build progress unknown", format!("{completed}/?"))
        } else {
            explicit(
                "Build",
                WidgetState::Empty,
                WidgetRole::Muted,
                "no active build",
            )
        };

        let parse_current = self.build.parse_current.unwrap_or(0);
        let parse = match (self.build.parse_current, self.build.parse_total) {
            (Some(current), Some(total)) if total > 0 => {
                if matches!(
                    self.build.status,
                    BuildStatus::Running | BuildStatus::Completed
                ) && current < total
                {
                    let mut projection =
                        GaugeProjection::determinate("Parse", current, total, WidgetRole::Warning);
                    projection.state = WidgetState::Partial;
                    projection.role = WidgetRole::Warning;
                    projection.detail = Some("phase ended before reported total".into());
                    projection
                } else if let Some(terminal) = build_terminal(self.build.status) {
                    terminal_progress("Parse", current, Some(total), terminal, "final")
                } else if matches!(
                    self.build.status,
                    BuildStatus::Running | BuildStatus::Completed
                ) && current >= total
                {
                    terminal_progress(
                        "Parse",
                        current,
                        Some(total),
                        WidgetTerminalState::Success,
                        "phase complete",
                    )
                } else {
                    GaugeProjection::determinate("Parse", current, total, WidgetRole::Progress)
                }
            }
            _ if self.build.status == BuildStatus::Parsing => GaugeProjection::indeterminate(
                "Parse progress unknown",
                format!("{parse_current}/?"),
            ),
            _ => explicit(
                "Parse",
                WidgetState::Unavailable,
                WidgetRole::Disabled,
                "not reported",
            ),
        };

        let runqueue = if let Some(terminal) = build_terminal(self.build.status) {
            terminal_progress("Runqueue", completed, total, terminal, "final")
        } else if self.build.status == BuildStatus::Running {
            total.map_or_else(
                || {
                    GaugeProjection::indeterminate(
                        "Runqueue progress unknown",
                        format!("{completed}/?"),
                    )
                },
                |total| {
                    GaugeProjection::determinate("Runqueue", completed, total, WidgetRole::Progress)
                },
            )
        } else {
            explicit(
                "Runqueue",
                WidgetState::Unavailable,
                WidgetRole::Disabled,
                "not active",
            )
        };

        let selected_task = match self
            .visible_task_row_refs_at(now)
            .get(self.task_progress_scroll)
            .copied()
        {
            Some(TaskRowRef::WaitingSummary(count)) => GaugeProjection::indeterminate(
                "Selected task progress unknown",
                format!("{count} waiting tasks"),
            ),
            Some(TaskRowRef::Task { task, state }) => {
                let label = format!("{}:{}", task.recipe, task.task);
                match (task.progress, state) {
                    (Some(progress), TaskState::Completed) => GaugeProjection::terminal(
                        label,
                        u64::from(progress),
                        100,
                        WidgetTerminalState::Success,
                        "task complete",
                    ),
                    (Some(progress), TaskState::Failed | TaskState::Lost) => {
                        GaugeProjection::terminal(
                            label,
                            u64::from(progress),
                            100,
                            WidgetTerminalState::Failure,
                            "task failed",
                        )
                    }
                    (Some(progress), TaskState::Cancelled) => GaugeProjection::terminal(
                        label,
                        u64::from(progress),
                        100,
                        WidgetTerminalState::Cancelled,
                        "task cancelled",
                    ),
                    (Some(progress), _) => GaugeProjection::determinate(
                        label,
                        u64::from(progress),
                        100,
                        WidgetRole::Progress,
                    ),
                    (None, TaskState::Active) => {
                        GaugeProjection::indeterminate(label, "progress unknown")
                    }
                    (None, _) => explicit(
                        &label,
                        WidgetState::Unavailable,
                        WidgetRole::Disabled,
                        "progress not reported",
                    ),
                }
            }
            None => explicit(
                "Selected task",
                WidgetState::Empty,
                WidgetRole::Muted,
                "no selection",
            ),
        };

        let selected_job = match self
            .job_history_rows()
            .get(self.build_history_selection)
            .copied()
        {
            Some(JobHistoryRowRef::Daemon(job)) => daemon_job_progress(job),
            Some(JobHistoryRowRef::Background(job)) => job.progress_projection(),
            Some(JobHistoryRowRef::Build(record)) => terminal_progress(
                record.target.as_deref().unwrap_or("Build record"),
                u64::try_from(record.completed_tasks).unwrap_or(u64::MAX),
                None,
                if record.success {
                    WidgetTerminalState::Success
                } else {
                    WidgetTerminalState::Failure
                },
                "retained build record",
            ),
            None => explicit(
                "Selected job",
                WidgetState::Empty,
                WidgetRole::Muted,
                "no selection",
            ),
        };

        let cpu = self
            .host_telemetry
            .cpu_utilization_percent
            .filter(|value| *value <= 100)
            .map_or_else(
                || {
                    explicit(
                        "CPU",
                        WidgetState::Unavailable,
                        WidgetRole::Disabled,
                        "sample missing",
                    )
                },
                |value| GaugeProjection::determinate("CPU", u64::from(value), 100, WidgetRole::Cpu),
            );
        let memory = utilization(
            self.host_telemetry.memory_total_bytes,
            self.host_telemetry.memory_available_bytes,
        )
        .map_or_else(
            || {
                explicit(
                    "RAM",
                    WidgetState::Unavailable,
                    WidgetRole::Disabled,
                    "sample missing or invalid",
                )
            },
            |value| GaugeProjection::determinate("RAM", value, 100, WidgetRole::Memory),
        );
        let build_filesystem = utilization(
            self.host_telemetry.disk_total_bytes,
            self.host_telemetry.disk_available_bytes,
        )
        .map_or_else(
            || {
                explicit(
                    "Build FS",
                    WidgetState::Unavailable,
                    WidgetRole::Disabled,
                    "sample missing or invalid",
                )
            },
            |value| GaugeProjection::determinate("Build FS", value, 100, WidgetRole::Progress),
        );

        let estimate = self
            .build
            .started
            .and_then(|started| now.duration_since(started).ok())
            .filter(|elapsed| elapsed.as_secs() > 0 && completed > 0)
            .map(|elapsed| {
                let seconds = elapsed.as_secs();
                let rate = completed.saturating_mul(600) / seconds;
                let eta = total.map(|total| {
                    Duration::from_secs(
                        total.saturating_sub(completed).saturating_mul(seconds) / completed,
                    )
                });
                ProgressEstimate {
                    average_tasks_per_minute_tenths: rate,
                    eta,
                }
            });

        ProgressHierarchy {
            build,
            parse,
            runqueue,
            selected_task,
            selected_job,
            resources: ResourceProgress {
                cpu,
                memory,
                build_filesystem,
            },
            sstate: match self.build.cache.summary.filter(|summary| summary.valid()) {
                Some(summary) if summary.wanted > 0 && !self.is_offline() => {
                    GaugeProjection::determinate(
                        "Sstate match",
                        summary.local + summary.mirrors,
                        summary.wanted,
                        WidgetRole::Progress,
                    )
                }
                _ => explicit(
                    "Sstate match",
                    WidgetState::Unavailable,
                    WidgetRole::Disabled,
                    "summary unavailable, stale, or no sstate requested",
                ),
            },
            estimate,
        }
    }
}
