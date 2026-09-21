use super::*;

#[test]
fn compatibility_version_selects_old_and_new_adapter_ranges_only_when_declared() {
    let map = VersionFallbackMap;
    let old = map.resolve_bitbake(
        &entry(CapabilityId::BitBakeWorkspaceInspection),
        Some("1.52.0"),
        &[direct(CapabilityProbeStatus::Inconclusive)],
    );
    assert!(matches!(
        old,
        VersionFallbackResolution::Inferred { implementation, .. }
            if implementation.id == "tinfoil.adapter.legacy"
    ));
    let new = map.resolve_bitbake(
        &entry(CapabilityId::BitBakeWorkspaceInspection),
        Some("2.18.0"),
        &[direct(CapabilityProbeStatus::Inconclusive)],
    );
    assert!(matches!(
        new,
        VersionFallbackResolution::Inferred { implementation, .. }
            if implementation.id == "tinfoil.adapter.modern"
    ));
    let undeclared = map.resolve_bitbake(&entry(CapabilityId::DevtoolUpgrade), Some("2.18"), &[]);
    assert!(matches!(
        undeclared,
        VersionFallbackResolution::Unknown { .. }
    ));
}
