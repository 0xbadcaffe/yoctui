use super::*;

#[test]
fn ux_logs_virtualized_window_and_source_time_filters_stay_bounded() {
    let mut logs = LogState::new(12_000, 2_000_000);
    for index in 0..10_000 {
        let mut entry = log(&format!("entry-{index:05}"));
        entry.path = Some(PathBuf::from(format!("/logs/source-{}", index % 3)));
        entry.timestamp = SystemTime::UNIX_EPOCH + Duration::from_secs(index as u64);
        logs.insert(entry);
    }
    logs.follow = false;
    logs.paused_len = Some(logs.entries.len());
    logs.selection = 9_000;
    let window = logs.window(7);
    assert_eq!(window.entries.len(), 7);
    assert_eq!(window.start, 8_994);
    assert_eq!(window.total, 10_000);
    assert_eq!(window.entries.last().unwrap().message, "entry-09000");

    logs.source_filter = Some(PathBuf::from("/logs/source-0"));
    logs.time_range = LogTimeRange::LastMinute;
    let filtered = logs.filtered().collect::<Vec<_>>();
    assert!(filtered.len() <= 21);
    assert!(
        filtered
            .iter()
            .all(|entry| entry.path.as_deref() == Some(Path::new("/logs/source-0")))
    );
    assert!(filtered.iter().all(|entry| {
        SystemTime::UNIX_EPOCH
            .checked_add(Duration::from_secs(9_999))
            .and_then(|latest| latest.duration_since(entry.timestamp).ok())
            .is_some_and(|age| age <= Duration::from_secs(60))
    }));
}
