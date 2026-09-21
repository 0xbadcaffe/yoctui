use super::*;

fn daemon_compatibility_snapshot(generation: u64) -> DaemonCompatibilitySnapshot {
    let environment = crate::YoctoEnvironmentIdentity {
        build_directory: crate::AuthoritativeValue::detected(
            "/work/poky/build".into(),
            crate::IdentityAuthority::InitializedEnvironment,
        ),
        ..crate::YoctoEnvironmentIdentity::default()
    };
    DaemonCompatibilitySnapshot {
        snapshot: CapabilitySnapshot {
            generation,
            environment,
            capabilities: vec![crate::CapabilityRecord {
                id: CapabilityId::BitBakeBuild,
                state: crate::CapabilityState::Available,
                evidence: vec![crate::CapabilityEvidence {
                    kind: crate::CapabilityEvidenceKind::DirectProbe,
                    outcome: crate::CapabilityEvidenceOutcome::Positive,
                    subject: "bitbake executable".into(),
                    detail: "The initialized environment exposes BitBake.".into(),
                    argv: Vec::new(),
                }],
            }],
        },
        implementations: BTreeMap::from([(
            CapabilityId::BitBakeBuild,
            CapabilityImplementation {
                id: "bitbake.build.command".into(),
                kind: crate::CapabilityImplementationKind::Command,
            },
        )]),
    }
}

mod daemon_state_partition_keeps_global_authority_and_client_presentation_distinct;

mod daemon_state_partition_rejects_unbounded_or_zero_collection_limits;

mod daemon_state_partition_fails_closed_when_revision_space_is_exhausted;

mod daemon_compatibility_owns_one_snapshot_and_rejects_stale_reprobes;

mod daemon_compatibility_requires_exact_implementation_for_enabled_records;

mod daemon_telemetry_defaults_are_safe_and_track_runtime_counts;

mod daemon_state_jobs_reuse_existing_typed_families_without_client_presentation;
