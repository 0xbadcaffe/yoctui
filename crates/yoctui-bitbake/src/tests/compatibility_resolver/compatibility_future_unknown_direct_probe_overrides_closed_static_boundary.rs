use super::*;

#[test]
fn compatibility_future_unknown_direct_probe_overrides_closed_static_boundary() {
    let catalog = CapabilityCatalog::builtin();
    let entry = catalog
        .entry(CapabilityId::BitBakeWorkspaceInspection)
        .unwrap();
    let resolved = CapabilityResolver::default().resolve(
        entry,
        Some("99.0"),
        &[observation(CapabilityProbeStatus::Positive, "workspace")],
    );
    assert_eq!(resolved.record.state, CapabilityState::Available);
    assert_eq!(resolved.implementation, Some(entry.preferred.clone()));
}
