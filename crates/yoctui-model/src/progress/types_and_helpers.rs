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

fn daemon_job_progress(job: &ClientDaemonJobSummary) -> GaugeProjection {
    let current = job.progress_current.unwrap_or(0);
    let valid_total = job
        .progress_total
        .filter(|total| *total > 0 && current <= *total);
    match job.lifecycle {
        ClientDaemonLifecycle::Running | ClientDaemonLifecycle::Connecting => valid_total
            .map_or_else(
                || GaugeProjection::indeterminate(job.label.clone(), "daemon job active"),
                |total| {
                    GaugeProjection::determinate(
                        job.label.clone(),
                        current,
                        total,
                        WidgetRole::Progress,
                    )
                },
            ),
        ClientDaemonLifecycle::Stopping => {
            GaugeProjection::indeterminate(job.label.clone(), "daemon job cancellation pending")
        }
        ClientDaemonLifecycle::Exited => terminal_progress(
            &job.label,
            current,
            valid_total,
            WidgetTerminalState::Success,
            "daemon job complete",
        ),
        ClientDaemonLifecycle::Failed | ClientDaemonLifecycle::Lost => terminal_progress(
            &job.label,
            current,
            valid_total,
            WidgetTerminalState::Failure,
            "daemon job failed",
        ),
        ClientDaemonLifecycle::Disconnected => explicit(
            &job.label,
            WidgetState::Unavailable,
            WidgetRole::Disabled,
            "daemon job unavailable",
        ),
    }
}

fn utilization(total: Option<u64>, available: Option<u64>) -> Option<u64> {
    let (total, available) = (total?, available?);
    if total == 0 || available > total {
        return None;
    }
    u64::try_from(u128::from(total - available) * 100 / u128::from(total)).ok()
}

