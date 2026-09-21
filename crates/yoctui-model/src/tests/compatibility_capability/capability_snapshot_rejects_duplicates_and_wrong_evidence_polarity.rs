use super::*;

#[test]
fn capability_snapshot_rejects_duplicates_and_wrong_evidence_polarity() {
    let duplicate = record(CapabilityId::WicCreate, CapabilityState::Available);
    let error = CapabilitySnapshot {
        generation: 1,
        environment: environment(),
        capabilities: vec![duplicate.clone(), duplicate],
    }
    .normalize()
    .unwrap_err();
    assert_eq!(
        error,
        CapabilityModelError::DuplicateCapability(CapabilityId::WicCreate)
    );

    let error = CapabilitySnapshot {
        generation: 1,
        environment: environment(),
        capabilities: vec![CapabilityRecord {
            id: CapabilityId::DevtoolUpgrade,
            state: CapabilityState::Unavailable {
                reason: reason("command.missing"),
            },
            evidence: vec![evidence(CapabilityEvidenceOutcome::Positive)],
        }],
    }
    .normalize()
    .unwrap_err();
    assert_eq!(
        error,
        CapabilityModelError::MissingEvidence {
            id: CapabilityId::DevtoolUpgrade,
            outcome: CapabilityEvidenceOutcome::Negative,
        }
    );
}
