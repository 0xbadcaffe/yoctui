use super::*;

#[test]
fn compatibility_utilities_unknown_snapshot_never_uses_host_path() {
    let statuses = utility_compatibility_statuses(None);
    assert!(
        statuses
            .iter()
            .filter(|status| status.id != "internal-workers")
            .all(|status| status.state == UtilityCompatibilityState::Unknown)
    );
    assert_eq!(
        statuses
            .iter()
            .find(|status| status.id == "internal-workers")
            .unwrap()
            .state,
        UtilityCompatibilityState::IntentionallyUnsupported
    );
}
