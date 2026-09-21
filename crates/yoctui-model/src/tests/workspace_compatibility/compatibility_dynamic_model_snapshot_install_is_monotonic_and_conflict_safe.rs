use super::*;

#[test]
fn compatibility_dynamic_model_snapshot_install_is_monotonic_and_conflict_safe() {
    let current = authority(
        4,
        vec![(
            CapabilityId::BitBakeBuild,
            CapabilityState::Available,
            Some("tinfoil.build"),
        )],
    );
    let mut state = WorkspaceCompatibilityState::default();
    assert_eq!(
        state.install(current.clone()).unwrap(),
        WorkspaceSnapshotInstall::Replaced
    );
    assert_eq!(
        state.install(current).unwrap(),
        WorkspaceSnapshotInstall::Unchanged
    );
    assert!(matches!(
        state.install(authority(
            3,
            vec![(
                CapabilityId::BitBakeBuild,
                CapabilityState::Available,
                Some("tinfoil.build")
            )]
        )),
        Err(WorkspaceCompatibilityError::StaleGeneration { .. })
    ));
    assert!(matches!(
        state.install(authority(
            4,
            vec![(
                CapabilityId::BitBakeBuild,
                CapabilityState::Unavailable {
                    reason: reason("same generation conflict")
                },
                None
            )]
        )),
        Err(WorkspaceCompatibilityError::ConflictingGeneration(4))
    ));
}
