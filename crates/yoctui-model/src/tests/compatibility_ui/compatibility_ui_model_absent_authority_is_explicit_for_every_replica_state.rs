use super::*;

#[test]
fn compatibility_ui_model_absent_authority_is_explicit_for_every_replica_state() {
    let compatibility = WorkspaceCompatibilityState::default();
    let state = CompatibilityUiState::default();
    for (replica, expected) in [
        (ClientReplicaStatus::Disconnected, "disconnected"),
        (ClientReplicaStatus::Synchronizing, "synchronizing"),
        (ClientReplicaStatus::Current, "no authoritative"),
        (ClientReplicaStatus::Stale, "stale"),
    ] {
        let projection = state.project(&compatibility, replica);
        let CompatibilityUiAuthorityStatus::Unavailable { reason } = projection.authority else {
            panic!("absent authority must not project as current");
        };
        assert!(reason.contains(expected));
        assert!(projection.environment.is_none());
        assert!(projection.rows.is_empty());
        assert_eq!(projection.summary, CapabilityAvailabilitySummary::default());
    }
}
