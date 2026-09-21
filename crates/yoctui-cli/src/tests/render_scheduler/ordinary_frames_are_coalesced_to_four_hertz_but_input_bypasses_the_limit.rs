use super::*;

#[test]
fn ordinary_frames_are_coalesced_to_four_hertz_but_input_bypasses_the_limit() {
    let start = std::time::Instant::now();
    let mut scheduler = RenderScheduler::default();
    assert!(scheduler.take_frame_at(start, ORDINARY_FRAME_INTERVAL));

    scheduler.invalidate(RenderCause::State);
    assert!(!scheduler.take_frame_at(
        start + std::time::Duration::from_millis(249),
        ORDINARY_FRAME_INTERVAL
    ));
    scheduler.invalidate(RenderCause::Telemetry);
    assert!(scheduler.take_frame_at(start + ORDINARY_FRAME_INTERVAL, ORDINARY_FRAME_INTERVAL));

    scheduler.invalidate(RenderCause::Input);
    assert!(scheduler.take_frame_at(
        start + ORDINARY_FRAME_INTERVAL + std::time::Duration::from_millis(1),
        ORDINARY_FRAME_INTERVAL
    ));
    assert_eq!(scheduler.metrics().frames, 3);
}
