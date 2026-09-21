use super::*;

#[test]
fn compatibility_probe_aggregation_allows_fallback_only_after_complete_inconclusive_probe() {
    let catalog = CapabilityCatalog::builtin();
    let entry = catalog.entry(CapabilityId::BitBakeBuild).unwrap();
    let resolver = CapabilityResolver::default();

    let missing = resolver.resolve(entry, Some("2.18"), &[]);
    assert!(matches!(
        missing.record.state,
        CapabilityState::Unknown { ref reason }
            if reason.code.as_str() == "evidence.missing"
    ));
    assert!(missing.implementation.is_none());

    let probed = resolver.resolve(
        entry,
        Some("2.18"),
        &[observation(
            CapabilityProbeStatus::Inconclusive,
            "backend handshake unavailable",
        )],
    );
    assert!(matches!(
        probed.record.state,
        CapabilityState::AvailableWithLimitations { .. }
    ));
    assert_eq!(probed.implementation.unwrap().id, "tinfoil.adapter.modern");
}
