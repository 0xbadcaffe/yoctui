use super::*;

#[test]
fn capability_snapshot_rejects_invalid_generation_reason_evidence_and_limitations() {
    let error = CapabilitySnapshot {
        generation: 0,
        environment: environment(),
        capabilities: Vec::new(),
    }
    .normalize()
    .unwrap_err();
    assert_eq!(error, CapabilityModelError::InvalidGeneration);

    assert!(CapabilityReasonCode::new("Command Missing").is_err());
    assert!(CapabilityReason::new("command.missing", "bad\nreason", None).is_err());

    let mut bad_evidence = evidence(CapabilityEvidenceOutcome::Positive);
    bad_evidence.argv = vec!["x".repeat(MAX_ENVIRONMENT_IDENTITY_TEXT_BYTES + 1)];
    let error = CapabilitySnapshot {
        generation: 1,
        environment: environment(),
        capabilities: vec![CapabilityRecord {
            id: CapabilityId::BitBakeBuild,
            state: CapabilityState::Available,
            evidence: vec![bad_evidence],
        }],
    }
    .normalize()
    .unwrap_err();
    assert_eq!(error, CapabilityModelError::InvalidEvidence);

    let error = CapabilitySnapshot {
        generation: 1,
        environment: environment(),
        capabilities: vec![record(
            CapabilityId::SpdxCreate,
            CapabilityState::AvailableWithLimitations {
                reason: reason("fallback.legacy_spdx"),
                limitations: Vec::new(),
            },
        )],
    }
    .normalize()
    .unwrap_err();
    assert_eq!(
        error,
        CapabilityModelError::InvalidLimitations(CapabilityId::SpdxCreate)
    );
}
