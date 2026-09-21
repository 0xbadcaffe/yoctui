use super::*;

#[test]
fn compatibility_future_unknown_absent_and_inconclusive_evidence_stays_unknown() {
    let catalog = CapabilityCatalog::builtin();
    let entry = catalog.entry(CapabilityId::BitBakeBuild).unwrap();
    for observations in [
        Vec::new(),
        vec![observation(CapabilityProbeStatus::Inconclusive, "build")],
    ] {
        let resolved = CapabilityResolver::default().resolve(entry, Some("3.0"), &observations);
        assert!(matches!(
            resolved.record.state,
            CapabilityState::Unknown { .. }
        ));
        assert!(resolved.implementation.is_none());
    }
}
