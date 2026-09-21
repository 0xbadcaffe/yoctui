use super::*;

#[test]
fn log_retention_prefers_important_diagnostics_and_reports_coalescing() {
    let mut logs = LogState::new(3, 1_000);
    logs.insert(tagged_log(
        "busybox",
        "do_compile",
        Severity::Warning,
        "warning retained",
    ));
    logs.insert(tagged_log(
        "busybox",
        "do_compile",
        Severity::Error,
        "error retained",
    ));
    for index in 0..20 {
        logs.insert(log(&format!("ordinary {index}")));
    }
    assert_eq!(logs.entries.len(), 3);
    assert!(
        logs.entries
            .iter()
            .any(|entry| entry.message == "warning retained")
    );
    assert!(
        logs.entries
            .iter()
            .any(|entry| entry.message == "error retained")
    );
    assert_eq!(logs.dropped_warnings, 0);
    assert_eq!(logs.dropped_errors, 0);

    logs.insert(log("repeat"));
    logs.insert(log("repeat"));
    assert_eq!(logs.coalesced, 1);
}
