use super::*;

#[test]
fn daemon_test_compare_diff_is_bounded_and_round_trips() {
    let diff = DaemonTestComparisonDiff {
        generation: 2,
        baseline: "a".into(),
        candidate: "b".into(),
        transitions: Vec::new(),
        limitations: vec!["limited".into()],
    }
    .bounded();
    let bytes = serde_json::to_vec(&diff).unwrap();
    assert_eq!(
        serde_json::from_slice::<DaemonTestComparisonDiff>(&bytes).unwrap(),
        diff
    );
}
