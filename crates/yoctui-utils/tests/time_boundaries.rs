use std::time::Duration;
use yoctui_utils::{format_duration, poll_timeout_ms, unix_ms};

#[test]
fn duration_formats_zero_and_discards_subseconds() {
    assert_eq!(format_duration(Duration::ZERO), "00:00:00");
    assert_eq!(format_duration(Duration::from_millis(999)), "00:00:00");
}

#[test]
fn duration_rolls_seconds_minutes_and_hours_without_wrapping_days() {
    for (seconds, expected) in [
        (59, "00:00:59"),
        (60, "00:01:00"),
        (3599, "00:59:59"),
        (3600, "01:00:00"),
        (86400, "24:00:00"),
        (360000, "100:00:00"),
    ] {
        assert_eq!(format_duration(Duration::from_secs(seconds)), expected);
    }
}

#[test]
fn duration_accepts_the_largest_representable_seconds() {
    assert_eq!(
        format_duration(Duration::from_secs(u64::MAX)),
        "5124095576030431:00:15"
    );
}

#[test]
fn polling_zero_and_submillisecond_timeouts_remain_positive() {
    assert_eq!(poll_timeout_ms(Duration::ZERO), 1);
    assert_eq!(poll_timeout_ms(Duration::from_nanos(1)), 1);
    assert_eq!(poll_timeout_ms(Duration::from_micros(999)), 1);
}

#[test]
fn polling_floors_submilliseconds_and_saturates_at_i32_max() {
    assert_eq!(poll_timeout_ms(Duration::from_micros(2500)), 2);
    assert_eq!(
        poll_timeout_ms(Duration::from_millis(i32::MAX as u64)),
        i32::MAX
    );
    assert_eq!(poll_timeout_ms(Duration::MAX), i32::MAX);
}

#[test]
fn unix_clock_is_bounded_by_surrounding_system_clock_reads() {
    let before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    let measured = u128::from(unix_ms());
    let after = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    assert!((before.min(after)..=before.max(after)).contains(&measured));
}
