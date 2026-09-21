use super::*;

#[test]
fn eviction_counts_dropped_warnings_and_errors() {
    let mut logs = LogState::new(1, 100);
    logs.insert(tagged_log(
        "busybox",
        "do_compile",
        Severity::Warning,
        "warning",
    ));
    logs.insert(tagged_log(
        "busybox",
        "do_compile",
        Severity::Error,
        "error",
    ));
    logs.insert(log("latest"));
    assert_eq!(logs.dropped, 2);
    assert_eq!(logs.dropped_warnings, 1);
    assert_eq!(logs.dropped_errors, 0);
    assert_eq!(
        logs.entries.front().map(|entry| entry.severity),
        Some(Severity::Error)
    );
}
