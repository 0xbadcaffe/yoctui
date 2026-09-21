use super::*;

#[test]
fn high_volume_logs_remain_within_retention_limits() {
    let mut logs = LogState::new(128, 4_096);
    for index in 0..20_000 {
        logs.insert(log(&format!("line {index}: {}", "x".repeat(index % 80))));
    }
    assert!(logs.entries.len() <= 128);
    assert!(logs.retained_bytes <= 4_096);
    assert_eq!(
        logs.retained_bytes,
        logs.entries.iter().map(|entry| entry.message.len()).sum()
    );
    assert!(logs.dropped > 0);
}
