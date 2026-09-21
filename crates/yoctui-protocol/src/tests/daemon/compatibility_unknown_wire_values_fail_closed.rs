use super::*;

#[test]
fn compatibility_unknown_wire_values_fail_closed() {
    let state: CompatibilityStateData =
        serde_json::from_str(r#"{"state":"available_in_a_future_protocol"}"#).unwrap();
    assert_eq!(state, CompatibilityStateData::UnknownWireState);
    assert!(!state.is_enabled());

    let evidence_kind: CompatibilityEvidenceKind =
        serde_json::from_str(r#""future_probe""#).unwrap();
    let evidence_outcome: CompatibilityEvidenceOutcome =
        serde_json::from_str(r#""future_outcome""#).unwrap();
    assert_eq!(evidence_kind, CompatibilityEvidenceKind::Unknown);
    assert_eq!(evidence_outcome, CompatibilityEvidenceOutcome::Unknown);
}
