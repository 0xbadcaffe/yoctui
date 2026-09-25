//! Coalesced frame invalidation for the interactive terminal client.

use yoctui_model::{App, BuildStatus, Screen, TaskState};

pub(crate) const ANIMATION_INTERVAL: std::time::Duration = std::time::Duration::from_millis(250);
pub(crate) const ELAPSED_REFRESH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);
pub(crate) const ORDINARY_FRAME_INTERVAL: std::time::Duration = ANIMATION_INTERVAL;
pub(crate) const SATURATED_FRAME_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);
const SATURATED_HOST_CPU_PERCENT: u8 = 90;

/// Whether the foreground workspace contains a visible indeterminate activity
/// glyph. Hidden work must not drive animation frames.
pub(crate) fn has_visible_indeterminate_activity(app: &App) -> bool {
    if !app.reduced_motion
        && (!app.background_activities.is_empty()
            || matches!(
                app.source_git_status,
                yoctui_model::SourceGitStatus::Scanning
            ))
    {
        return true;
    }

    if app.reduced_motion
        || app.active_dialog().is_some()
        || app.menu.is_open()
        || app.command_palette_open
    {
        return false;
    }

    let jobs = app.job_summary();
    if app
        .transient_status()
        .is_some_and(|status| status.kind == yoctui_model::TransientStatusKind::Activity)
        && (app.daemon.bitbake == yoctui_model::ClientDaemonLifecycle::Connecting
            || jobs.active > 0
            || jobs.queued > 0)
    {
        return true;
    }

    if !matches!(app.screen, Screen::Dashboard | Screen::Tasks)
        || !matches!(
            app.build.status,
            BuildStatus::LoadingWorkspace
                | BuildStatus::Parsing
                | BuildStatus::Running
                | BuildStatus::Cancelling
        )
    {
        return false;
    }

    app.build.total.is_none()
        || app
            .tasks
            .values()
            .any(|task| task.state == TaskState::Active && task.progress.is_none())
}

pub(crate) fn has_live_elapsed_time(app: &App) -> bool {
    matches!(
        app.build.status,
        BuildStatus::LoadingWorkspace
            | BuildStatus::Parsing
            | BuildStatus::Running
            | BuildStatus::Cancelling
    )
}

pub(crate) fn ordinary_frame_interval(app: &App) -> std::time::Duration {
    if has_live_elapsed_time(app)
        && app
            .host_telemetry
            .cpu_utilization_percent
            .is_some_and(|cpu| cpu >= SATURATED_HOST_CPU_PERCENT)
    {
        SATURATED_FRAME_INTERVAL
    } else {
        ORDINARY_FRAME_INTERVAL
    }
}

pub(crate) fn animation_interval(app: &App) -> std::time::Duration {
    ordinary_frame_interval(app)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RenderCause {
    Initial,
    Input,
    State,
    Telemetry,
    Presentation,
    Resize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct RenderMetrics {
    pub(crate) requests: u64,
    pub(crate) frames: u64,
    pub(crate) coalesced: u64,
    pub(crate) skipped_checks: u64,
}

#[derive(Debug)]
pub(crate) struct RenderScheduler {
    pending: bool,
    urgent: bool,
    metrics: RenderMetrics,
    last_cause: Option<RenderCause>,
    last_frame: Option<std::time::Instant>,
}

impl Default for RenderScheduler {
    fn default() -> Self {
        let mut scheduler = Self {
            pending: false,
            urgent: false,
            metrics: RenderMetrics::default(),
            last_cause: None,
            last_frame: None,
        };
        scheduler.invalidate(RenderCause::Initial);
        scheduler
    }
}

impl RenderScheduler {
    pub(crate) fn invalidate(&mut self, cause: RenderCause) {
        self.metrics.requests = self.metrics.requests.saturating_add(1);
        if self.pending {
            self.metrics.coalesced = self.metrics.coalesced.saturating_add(1);
        }
        self.pending = true;
        self.urgent |= matches!(
            cause,
            RenderCause::Initial | RenderCause::Input | RenderCause::Resize
        );
        self.last_cause = Some(cause);
    }

    pub(crate) fn invalidate_if(&mut self, changed: bool, cause: RenderCause) {
        if changed {
            self.invalidate(cause);
        }
    }

    #[cfg(test)]
    pub(crate) fn take_frame(&mut self) -> bool {
        self.take_frame_with_interval(ORDINARY_FRAME_INTERVAL)
    }

    pub(crate) fn take_frame_with_interval(&mut self, interval: std::time::Duration) -> bool {
        self.take_frame_at(std::time::Instant::now(), interval)
    }

    fn take_frame_at(&mut self, now: std::time::Instant, interval: std::time::Duration) -> bool {
        if !self.pending {
            self.metrics.skipped_checks = self.metrics.skipped_checks.saturating_add(1);
            return false;
        }
        if !self.urgent
            && self
                .last_frame
                .is_some_and(|last| now.saturating_duration_since(last) < interval)
        {
            self.metrics.skipped_checks = self.metrics.skipped_checks.saturating_add(1);
            return false;
        }
        self.pending = false;
        self.urgent = false;
        self.last_frame = Some(now);
        self.metrics.frames = self.metrics.frames.saturating_add(1);
        true
    }

    pub(crate) fn metrics(&self) -> RenderMetrics {
        self.metrics
    }

    #[cfg(test)]
    pub(crate) fn last_cause(&self) -> Option<RenderCause> {
        self.last_cause
    }
}

#[cfg(test)]
#[path = "tests/render_scheduler/mod.rs"]
mod tests;
