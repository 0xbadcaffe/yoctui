use super::*;

#[test]
fn compatibility_snapshot_round_trips_bounded_identity_state_and_evidence() {
    let snapshot = compatibility_snapshot_fixture(7);
    snapshot.validate().unwrap();

    let encoded = serde_json::to_vec(&snapshot).unwrap();
    let decoded: CompatibilitySnapshotData = serde_json::from_slice(&encoded).unwrap();

    assert_eq!(decoded, snapshot);
    assert!(decoded.capabilities[0].state.is_enabled());
}
