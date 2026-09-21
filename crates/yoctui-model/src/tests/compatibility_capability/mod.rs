use super::*;

fn environment() -> YoctoEnvironmentIdentity {
    YoctoEnvironmentIdentity {
        build_directory: AuthoritativeValue::detected(
            "/work/build".into(),
            IdentityAuthority::InitializedEnvironment,
        ),
        bitbake_version: AuthoritativeValue::detected(
            "2.8.1".into(),
            IdentityAuthority::BitBakeVersionProbe,
        ),
        ..YoctoEnvironmentIdentity::default()
    }
}

fn reason(code: &str) -> CapabilityReason {
    CapabilityReason::new(code, "Exact environment-correlated reason", None).unwrap()
}

fn evidence(outcome: CapabilityEvidenceOutcome) -> CapabilityEvidence {
    CapabilityEvidence {
        kind: CapabilityEvidenceKind::DirectProbe,
        outcome,
        subject: "devtool --help".into(),
        detail: "Bounded help probe inspected the upgrade subcommand".into(),
        argv: vec!["/work/bin/devtool".into(), "--help".into()],
    }
}

fn record(id: CapabilityId, state: CapabilityState) -> CapabilityRecord {
    let outcome = match state {
        CapabilityState::Available | CapabilityState::AvailableWithLimitations { .. } => {
            CapabilityEvidenceOutcome::Positive
        }
        CapabilityState::Unavailable { .. } => CapabilityEvidenceOutcome::Negative,
        CapabilityState::Unknown { .. } | CapabilityState::Unsupported { .. } => {
            CapabilityEvidenceOutcome::Inconclusive
        }
    };
    CapabilityRecord {
        id,
        state,
        evidence: vec![evidence(outcome)],
    }
}

mod capability_snapshot_normalizes_all_states_and_supports_fail_closed_lookup;

mod capability_inventory_is_unique_behavior_oriented_and_complete;

mod capability_snapshot_rejects_duplicates_and_wrong_evidence_polarity;

mod capability_snapshot_rejects_invalid_generation_reason_evidence_and_limitations;

mod capability_snapshot_keeps_exact_environment_association;

mod compatibility_older_release_preserves_mixed_state_without_global_failure;
