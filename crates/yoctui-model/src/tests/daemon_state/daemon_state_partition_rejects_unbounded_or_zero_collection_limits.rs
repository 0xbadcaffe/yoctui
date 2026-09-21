use super::*;

#[test]
fn daemon_state_partition_rejects_unbounded_or_zero_collection_limits() {
    for limits in [
        DaemonStateLimits {
            logs: 0,
            ..DaemonStateLimits::default()
        },
        DaemonStateLimits {
            errors: MAX_DAEMON_COLLECTION_LIMIT + 1,
            ..DaemonStateLimits::default()
        },
    ] {
        assert!(matches!(
            DaemonGlobalState::new(DaemonModelInstanceId([0; 16]), 0, "boot".into(), limits),
            Err(DaemonStateError::InvalidLimit { .. })
        ));
    }
}
