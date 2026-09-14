use std::time::{Duration, SystemTime};

pub fn format_duration(duration: Duration) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        duration.as_secs() / 3600,
        duration.as_secs() / 60 % 60,
        duration.as_secs() % 60
    )
}

/// Current Unix milliseconds, saturating for out-of-range system clocks.
pub fn unix_ms() -> u64 {
    SystemTime::UNIX_EPOCH
        .elapsed()
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

/// A positive bounded poll timeout; callers handle nonblocking zero separately.
pub fn poll_timeout_ms(timeout: Duration) -> i32 {
    timeout.as_millis().clamp(1, i32::MAX as u128) as i32
}

#[cfg(test)]
mod tests {
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
}
