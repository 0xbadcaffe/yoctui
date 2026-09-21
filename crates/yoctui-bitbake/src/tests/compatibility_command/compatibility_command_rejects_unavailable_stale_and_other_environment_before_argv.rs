use super::*;

#[test]
fn compatibility_command_rejects_unavailable_stale_and_other_environment_before_argv() {
    let reason = CapabilityReason::new(
        "command.option_missing",
        "Current BitBake help does not expose -g.",
        Some("Required option: -g".into()),
    )
    .unwrap();
    let mut unavailable = authority(
        8,
        &[(
            CapabilityId::BitBakeGraphGeneration,
            BITBAKE_GRAPH_ARGV_IMPLEMENTATION,
        )],
    );
    unavailable.snapshot.capabilities[0] = CapabilityRecord {
        id: CapabilityId::BitBakeGraphGeneration,
        state: CapabilityState::Unavailable { reason },
        evidence: vec![CapabilityEvidence {
            kind: CapabilityEvidenceKind::DirectProbe,
            outcome: CapabilityEvidenceOutcome::Negative,
            subject: "bitbake --help".into(),
            detail: "The -g option is absent.".into(),
            argv: vec!["bitbake".into(), "--help".into()],
        }],
    };
    unavailable.implementations.clear();
    let unavailable = unavailable.normalize().unwrap();
    assert!(matches!(
        planner(&unavailable).dependency_graph("busybox"),
        Err(BitBakeCommandAuthorizationError::Unavailable { reason, .. })
            if reason.contains("does not expose -g")
    ));
    assert!(matches!(
        BitBakeCommandPlanner::new(&unavailable, 7, Path::new("/work/build")),
        Err(BitBakeCommandAuthorizationError::StaleGeneration { .. })
    ));
    assert!(matches!(
        BitBakeCommandPlanner::new(&unavailable, 8, Path::new("/other/build")),
        Err(BitBakeCommandAuthorizationError::EnvironmentMismatch)
    ));
}
