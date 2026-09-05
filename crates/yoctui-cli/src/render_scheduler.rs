//! Coalesced frame invalidation for the interactive terminal client.

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
