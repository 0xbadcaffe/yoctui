use super::*;

#[test]
fn four_hertz_live_budget_coalesces_many_updates_per_frame() {
    let start = std::time::Instant::now();
    let mut scheduler = RenderScheduler::default();
    assert!(scheduler.take_frame_at(start, ORDINARY_FRAME_INTERVAL));
    for _ in 0..10 {
        for _ in 0..64 {
            scheduler.invalidate(RenderCause::State);
        }
        assert!(scheduler.take_frame_at(
            start + ORDINARY_FRAME_INTERVAL * (scheduler.metrics().frames as u32),
            ORDINARY_FRAME_INTERVAL
        ));
    }
    let metrics = scheduler.metrics();
    assert_eq!(metrics.frames, 11);
    assert_eq!(metrics.requests, 641);
    assert_eq!(metrics.coalesced, 630);
}
