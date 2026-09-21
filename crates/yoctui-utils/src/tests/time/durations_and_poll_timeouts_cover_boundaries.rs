use super::*;

#[test]
fn durations_and_poll_timeouts_cover_boundaries() {
    assert_eq!(format_duration(Duration::ZERO), "00:00:00");
    assert_eq!(format_duration(Duration::from_secs(360_061)), "100:01:01");
    assert_eq!(poll_timeout_ms(Duration::ZERO), 1);
    assert_eq!(poll_timeout_ms(Duration::from_nanos(1)), 1);
    assert_eq!(poll_timeout_ms(Duration::from_millis(42)), 42);
    assert_eq!(poll_timeout_ms(Duration::MAX), i32::MAX);
}
