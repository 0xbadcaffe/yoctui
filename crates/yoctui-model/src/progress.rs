//! Typed hierarchical progress projection over existing reducer authority.

use crate::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityLifecycle {
    Loading,
    Running,
    Waiting,
    Succeeded,
    Failed,
    Cancelled,
}

impl ActivityLifecycle {
    pub const fn word(self) -> &'static str {
        match self {
            Self::Loading => "loading",
            Self::Running => "running",
            Self::Waiting => "waiting",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }

    pub const fn active(self) -> bool {
        matches!(self, Self::Loading | Self::Running | Self::Waiting)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ActivityProjection {
    pub lifecycle: ActivityLifecycle,
    pub phase: Option<usize>,
}

impl ActivityProjection {
    pub const SYMBOL_COUNT: usize = 6;

    pub fn new(
        lifecycle: ActivityLifecycle,
        tick: u64,
        speed: AnimationSpeed,
        reduced_motion: bool,
    ) -> Self {
        let divisor = match speed {
            AnimationSpeed::Fast => 1,
            AnimationSpeed::Slow => 3,
        };
        let phase = (lifecycle.active() && !reduced_motion).then(|| {
            usize::try_from(tick / divisor)
                .unwrap_or(usize::MAX)
                .wrapping_rem(Self::SYMBOL_COUNT)
        });
        Self { lifecycle, phase }
    }

    pub fn text(self) -> String {
        if self.phase.is_some() {
            format!("{} active", self.lifecycle.word())
        } else {
            self.lifecycle.word().into()
        }
    }
}

impl App {
    pub fn activity_projection(&self, lifecycle: ActivityLifecycle) -> ActivityProjection {
        ActivityProjection::new(
            lifecycle,
            self.animation_frame,
            self.animation_speed,
            self.reduced_motion,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressEstimate {
    pub average_tasks_per_minute_tenths: u64,
    pub eta: Option<Duration>,
}

impl ProgressEstimate {
    pub fn text(&self) -> String {
        format!(
            "estimate avg {}.{}/m · ETA {}",
            self.average_tasks_per_minute_tenths / 10,
            self.average_tasks_per_minute_tenths % 10,
            self.eta.map(format_duration).unwrap_or_else(|| "--".into())
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceProgress {
    pub cpu: GaugeProjection,
    pub memory: GaugeProjection,
    pub build_filesystem: GaugeProjection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgressHierarchy {
    pub build: GaugeProjection,
    pub parse: GaugeProjection,
    pub runqueue: GaugeProjection,
    pub selected_task: GaugeProjection,
    pub selected_job: GaugeProjection,
    pub resources: ResourceProgress,
    pub sstate: GaugeProjection,
    pub estimate: Option<ProgressEstimate>,
}

fn explicit(label: &str, state: WidgetState, role: WidgetRole, detail: &str) -> GaugeProjection {
    GaugeProjection::explicit(label, state, role, detail)
}

fn terminal_progress(
    label: &str,
    current: u64,
    total: Option<u64>,
    terminal: WidgetTerminalState,
    detail: &str,
) -> GaugeProjection {
    GaugeProjection::terminal(label, current, total.unwrap_or(0), terminal, detail)
}

fn build_terminal(status: BuildStatus) -> Option<WidgetTerminalState> {
    match status {
        BuildStatus::Completed => Some(WidgetTerminalState::Success),
        BuildStatus::Cancelled => Some(WidgetTerminalState::Cancelled),
        BuildStatus::Failed => Some(WidgetTerminalState::Failure),
        _ => None,
    }
}

fn utilization(total: Option<u64>, available: Option<u64>) -> Option<u64> {
    let (total, available) = (total?, available?);
    if total == 0 || available > total {
        return None;
    }
    u64::try_from(u128::from(total - available) * 100 / u128::from(total)).ok()
}

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
            sstate: explicit(
                "Sstate reuse",
                WidgetState::Unavailable,
                WidgetRole::Disabled,
                "backend does not report progress",
            ),
            estimate,
        }
    }
}

impl BackgroundJob {
    pub fn progress_projection(&self) -> GaugeProjection {
        let terminal = match self.status {
            BackgroundJobStatus::Succeeded => Some(WidgetTerminalState::Success),
            BackgroundJobStatus::Failed | BackgroundJobStatus::Lost => {
                Some(WidgetTerminalState::Failure)
            }
            BackgroundJobStatus::Cancelled => Some(WidgetTerminalState::Cancelled),
            _ => None,
        };
        match (&self.progress, terminal) {
            (BackgroundJobProgress::Percent(value), Some(terminal)) => {
                GaugeProjection::terminal(&self.title, u64::from(*value), 100, terminal, "final")
            }
            (BackgroundJobProgress::Units { completed, total }, Some(terminal)) => {
                GaugeProjection::terminal(&self.title, *completed, *total, terminal, "final")
            }
            (BackgroundJobProgress::Percent(value), None) => GaugeProjection::determinate(
                &self.title,
                u64::from(*value),
                100,
                WidgetRole::Progress,
            ),
            (BackgroundJobProgress::Units { completed, total }, None) => {
                GaugeProjection::determinate(&self.title, *completed, *total, WidgetRole::Progress)
            }
            (BackgroundJobProgress::Indeterminate, Some(terminal)) => {
                terminal_progress(&self.title, 0, None, terminal, "progress not reported")
            }
            (BackgroundJobProgress::Indeterminate, None) => {
                GaugeProjection::indeterminate(&self.title, "progress unknown")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ux_throbber_phase_is_reducer_owned_and_terminal_states_never_animate() {
        let fast =
            ActivityProjection::new(ActivityLifecycle::Running, 7, AnimationSpeed::Fast, false);
        assert_eq!(fast.phase, Some(1));
        let slow =
            ActivityProjection::new(ActivityLifecycle::Loading, 7, AnimationSpeed::Slow, false);
        assert_eq!(slow.phase, Some(2));
        let reduced = ActivityProjection::new(
            ActivityLifecycle::Waiting,
            u64::MAX,
            AnimationSpeed::Fast,
            true,
        );
        assert_eq!(reduced.phase, None);
        assert_eq!(reduced.text(), "waiting");
        for lifecycle in [
            ActivityLifecycle::Succeeded,
            ActivityLifecycle::Failed,
            ActivityLifecycle::Cancelled,
        ] {
            let terminal =
                ActivityProjection::new(lifecycle, u64::MAX, AnimationSpeed::Fast, false);
            assert_eq!(terminal.phase, None);
            assert_eq!(terminal.text(), lifecycle.word());
        }
    }

    #[test]
    fn ux_progress_separates_build_parse_runqueue_resources_and_sstate() {
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(600);
        let mut app = App::new(16, 4_096);
        app.build.status = BuildStatus::Running;
        app.build.started = Some(now - Duration::from_secs(120));
        app.build.completed = 30;
        app.build.total = Some(100);
        app.build.parse_current = Some(20);
        app.build.parse_total = Some(20);
        app.host_telemetry.cpu_utilization_percent = Some(72);
        app.host_telemetry.memory_total_bytes = Some(1_000);
        app.host_telemetry.memory_available_bytes = Some(250);
        app.host_telemetry.disk_total_bytes = Some(2_000);
        app.host_telemetry.disk_available_bytes = Some(1_000);

        let hierarchy = app.progress_hierarchy_at(now);
        assert_eq!(
            hierarchy.build.fraction.unwrap().exact_text(),
            "30/100 (30%)"
        );
        assert_eq!(hierarchy.parse.state, WidgetState::TerminalSuccess);
        assert_eq!(hierarchy.runqueue.fraction.unwrap().current, 30);
        assert_eq!(hierarchy.resources.cpu.fraction.unwrap().current, 72);
        assert_eq!(hierarchy.resources.memory.fraction.unwrap().current, 75);
        assert_eq!(
            hierarchy
                .resources
                .build_filesystem
                .fraction
                .unwrap()
                .current,
            50
        );
        assert_eq!(hierarchy.sstate.state, WidgetState::Unavailable);
        assert!(
            hierarchy
                .estimate
                .unwrap()
                .text()
                .starts_with("estimate avg")
        );
    }

    #[test]
    fn ux_progress_preserves_unknown_totals_and_terminal_job_progress() {
        let mut app = App::new(16, 4_096);
        app.build.status = BuildStatus::Running;
        app.build.completed = 12;
        let hierarchy = app.progress_hierarchy_at(SystemTime::UNIX_EPOCH);
        assert_eq!(hierarchy.build.state, WidgetState::Active);
        assert!(hierarchy.build.text(false, true).contains("12/?"));
        assert!(
            hierarchy
                .runqueue
                .text(false, true)
                .contains("progress unknown")
        );

        let job = BackgroundJob {
            id: BackgroundJobId(1),
            kind: BackgroundJobKind::Build,
            title: "SDK".into(),
            status: BackgroundJobStatus::Failed,
            context: BackgroundJobContext::default(),
            cancellation_supported: true,
            progress: BackgroundJobProgress::Units {
                completed: 7,
                total: 10,
            },
            output: VecDeque::new(),
            retained_output_bytes: 0,
            dropped_output_entries: 0,
            warnings: 0,
            errors: 1,
            queued_at: SystemTime::UNIX_EPOCH,
            started_at: Some(SystemTime::UNIX_EPOCH),
            finished_at: Some(SystemTime::UNIX_EPOCH),
            result: None,
            error: None,
        };
        let projection = job.progress_projection();
        assert_eq!(projection.state, WidgetState::TerminalFailure);
        assert_eq!(projection.fraction.unwrap().exact_text(), "7/10 (70%)");
    }
}
