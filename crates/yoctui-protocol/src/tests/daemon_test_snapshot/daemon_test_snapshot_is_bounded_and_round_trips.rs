use super::*;

#[test]
fn daemon_test_snapshot_is_bounded_and_round_trips() {
    let snapshot = DaemonTestResultSnapshot {
        generation: 4,
        records: (0..(MAX_TEST_RESULT_RECORDS + 2))
            .map(|index| DaemonTestResultRecord {
                identity: index.to_string(),
                outcome: "pass".into(),
                duration_ms: None,
                log_path: None,
            })
            .collect(),
        limitations: (0..(MAX_TEST_RESULT_LIMITATIONS + 2))
            .map(|index| index.to_string())
            .collect(),
        complete: true,
    }
    .bounded();
    assert_eq!(snapshot.records.len(), MAX_TEST_RESULT_RECORDS);
    assert_eq!(snapshot.limitations.len(), MAX_TEST_RESULT_LIMITATIONS);
    let encoded = serde_json::to_vec(&snapshot).unwrap();
    let decoded: DaemonTestResultSnapshot = serde_json::from_slice(&encoded).unwrap();
    assert_eq!(decoded, snapshot);
}
