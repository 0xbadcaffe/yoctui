use super::*;

#[test]
fn unchanged_sources_do_not_invalidate() {
    let mut scheduler = RenderScheduler::default();
    assert!(scheduler.take_frame());
    scheduler.invalidate_if(false, RenderCause::State);
    assert!(!scheduler.take_frame());
    assert_eq!(scheduler.metrics().requests, 1);
    assert_eq!(scheduler.metrics().skipped_checks, 1);
}
