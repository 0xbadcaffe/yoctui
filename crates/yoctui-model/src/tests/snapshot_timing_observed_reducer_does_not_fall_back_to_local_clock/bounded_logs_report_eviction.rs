use super::*;

#[test]
fn bounded_logs_report_eviction() {
    let mut l = LogState::new(2, 100);
    l.insert(log("a"));
    l.insert(log("b"));
    l.insert(log("c"));
    assert_eq!(l.entries.len(), 2);
    assert_eq!(l.dropped, 1)
}
