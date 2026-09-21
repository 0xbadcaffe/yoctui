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
