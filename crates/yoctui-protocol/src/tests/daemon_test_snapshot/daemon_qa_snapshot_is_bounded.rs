use super::*;

#[test]
fn daemon_qa_snapshot_is_bounded() {
    let snapshot = DaemonQaSnapshot {
        generation: 1,
        capability: "available".into(),
        task_bindings: (0..MAX_QA_RECORDS + 1).map(|i| i.to_string()).collect(),
        reports: (0..MAX_QA_RECORDS + 1).map(|i| i.to_string()).collect(),
        limitations: Vec::new(),
    }
    .bounded();
    assert_eq!(snapshot.task_bindings.len(), MAX_QA_RECORDS);
    assert_eq!(snapshot.reports.len(), MAX_QA_RECORDS);
}
