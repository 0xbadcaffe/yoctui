use super::*;

#[test]
fn daemon_compatibility_requires_exact_implementation_for_enabled_records() {
    let mut missing = daemon_compatibility_snapshot(1);
    missing.implementations.clear();
    assert_eq!(
        missing.normalize(),
        Err(DaemonStateError::CompatibilityImplementationMismatch(
            CapabilityId::BitBakeBuild
        ))
    );

    let mut disabled = daemon_compatibility_snapshot(1);
    disabled.snapshot.capabilities[0].state = crate::CapabilityState::Unknown {
        reason: crate::CapabilityReason::new(
            "probe.unknown",
            "The capability probe did not return conclusive evidence.",
            None,
        )
        .unwrap(),
    };
    assert_eq!(
        disabled.normalize(),
        Err(DaemonStateError::CompatibilityImplementationMismatch(
            CapabilityId::BitBakeBuild
        ))
    );
}
