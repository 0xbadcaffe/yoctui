//! Coalesced frame invalidation for the interactive terminal client.

use yoctui_model::{App, BuildStatus, Screen, TaskState};

pub(crate) const ANIMATION_INTERVAL: std::time::Duration = std::time::Duration::from_millis(200);
pub(crate) const ELAPSED_REFRESH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

/// Whether the foreground workspace contains a visible indeterminate activity
/// glyph. Hidden work must not drive animation frames.
pub(crate) fn has_visible_indeterminate_activity(app: &App) -> bool {
    if app.reduced_motion
        || app.active_dialog().is_some()
        || app.menu.is_open()
        || app.command_palette_open
        || !matches!(app.screen, Screen::Dashboard | Screen::Tasks)
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
    metrics: RenderMetrics,
    last_cause: Option<RenderCause>,
}

impl Default for RenderScheduler {
    fn default() -> Self {
        let mut scheduler = Self {
            pending: false,
            metrics: RenderMetrics::default(),
            last_cause: None,
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
        self.last_cause = Some(cause);
    }

    pub(crate) fn invalidate_if(&mut self, changed: bool, cause: RenderCause) {
        if changed {
            self.invalidate(cause);
        }
    }

    pub(crate) fn take_frame(&mut self) -> bool {
        if !self.pending {
            self.metrics.skipped_checks = self.metrics.skipped_checks.saturating_add(1);
            return false;
        }
        self.pending = false;
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
mod tests {
    use super::*;
    use yoctui_model::{Dialog, TaskId, TaskInfo};

    #[test]
    fn render_requests_coalesce_and_idle_checks_do_not_create_frames() {
        let mut scheduler = RenderScheduler::default();
        scheduler.invalidate(RenderCause::State);
        scheduler.invalidate(RenderCause::Telemetry);
        assert!(scheduler.take_frame());
        assert!(!scheduler.take_frame());
        assert!(!scheduler.take_frame());
        assert_eq!(
            scheduler.metrics(),
            RenderMetrics {
                requests: 3,
                frames: 1,
                coalesced: 2,
                skipped_checks: 2,
            }
        );
    }

    #[test]
    fn input_is_an_immediate_invalidation_independent_of_ticks() {
        let mut scheduler = RenderScheduler::default();
        assert!(scheduler.take_frame());
        scheduler.invalidate(RenderCause::Input);
        assert_eq!(scheduler.last_cause(), Some(RenderCause::Input));
        assert!(scheduler.take_frame());
        assert_eq!(scheduler.metrics().frames, 2);
    }

    #[test]
    fn unchanged_sources_do_not_invalidate() {
        let mut scheduler = RenderScheduler::default();
        assert!(scheduler.take_frame());
        scheduler.invalidate_if(false, RenderCause::State);
        assert!(!scheduler.take_frame());
        assert_eq!(scheduler.metrics().requests, 1);
        assert_eq!(scheduler.metrics().skipped_checks, 1);
    }

    #[test]
    fn animation_is_visible_only_indeterminate_and_nonterminal() {
        let mut app = App::new(16, 16 * 1024);
        app.build.status = BuildStatus::Running;
        assert!(has_visible_indeterminate_activity(&app));

        app.screen = Screen::Recipes;
        assert!(!has_visible_indeterminate_activity(&app));

        app.screen = Screen::Tasks;
        app.build.total = Some(100);
        assert!(!has_visible_indeterminate_activity(&app));

        let task = TaskInfo::active(TaskId("busy".into()), "busybox".into(), "do_compile".into());
        app.tasks.insert(task.id.clone(), task);
        assert!(has_visible_indeterminate_activity(&app));

        app.build.status = BuildStatus::Completed;
        assert!(!has_visible_indeterminate_activity(&app));
    }

    #[test]
    fn overlays_and_reduced_motion_freeze_animation_but_not_elapsed_time() {
        let mut app = App::new(16, 16 * 1024);
        app.build.status = BuildStatus::Running;
        assert!(has_live_elapsed_time(&app));

        app.reduced_motion = true;
        assert!(!has_visible_indeterminate_activity(&app));
        assert!(has_live_elapsed_time(&app));

        app.reduced_motion = false;
        app.dialogs.push_back(Dialog::BuildCompletion);
        assert!(!has_visible_indeterminate_activity(&app));

        app.dialogs.clear();
        app.command_palette_open = true;
        assert!(!has_visible_indeterminate_activity(&app));
    }

    #[test]
    fn presentation_cadences_are_explicitly_bounded() {
        assert!(ANIMATION_INTERVAL >= std::time::Duration::from_millis(100));
        assert!(ANIMATION_INTERVAL <= std::time::Duration::from_millis(250));
        assert_eq!(ELAPSED_REFRESH_INTERVAL, std::time::Duration::from_secs(1));
    }

    #[test]
    fn ten_hertz_live_budget_coalesces_many_updates_per_frame() {
        let mut scheduler = RenderScheduler::default();
        assert!(scheduler.take_frame());
        for _ in 0..10 {
            for _ in 0..64 {
                scheduler.invalidate(RenderCause::State);
            }
            assert!(scheduler.take_frame());
        }
        let metrics = scheduler.metrics();
        assert_eq!(metrics.frames, 11);
        assert_eq!(metrics.requests, 641);
        assert_eq!(metrics.coalesced, 630);
    }
}
