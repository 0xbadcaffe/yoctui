use super::*;

#[test]
fn ux_internal_log_state_is_bounded_filterable_and_viewport_only() {
    let mut logs = InternalLogState::new(128, 64 * 1024);
    for index in 0..10_000 {
        logs.insert(record(
            index,
            if index % 7 == 0 {
                InternalLogLevel::Warning
            } else {
                InternalLogLevel::Debug
            },
            if index % 2 == 0 {
                "yoctui::runtime"
            } else {
                "yoctui::adapter"
            },
        ));
    }
    assert_eq!(logs.entries.len(), 128);
    assert!(logs.retained_bytes <= logs.max_bytes);
    assert_eq!(logs.evicted, 10_000 - 128);
    assert_eq!(logs.window(6).entries.len(), 6);

    logs.level_filter = Some(InternalLogLevel::Warning);
    logs.target_filter = Some("yoctui::runtime".into());
    logs.query = "diagnostic".into();
    assert!(logs.filtered().all(|entry| {
        entry.level == InternalLogLevel::Warning && entry.target == "yoctui::runtime"
    }));
    logs.note_ingress_dropped(9);
    assert_eq!(logs.ingress_dropped, 9);
}
