use super::*;

#[test]
fn input_is_an_immediate_invalidation_independent_of_ticks() {
    let mut scheduler = RenderScheduler::default();
    assert!(scheduler.take_frame());
    scheduler.invalidate(RenderCause::Input);
    assert_eq!(scheduler.last_cause(), Some(RenderCause::Input));
    assert!(scheduler.take_frame());
    assert_eq!(scheduler.metrics().frames, 2);
}
