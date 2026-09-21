use super::*;

#[test]
fn compatibility_version_catalog_fallback_is_not_an_executable_probe() {
    let capability = entry(CapabilityId::BitBakeBuild);
    assert!(
        capability
            .probes
            .iter()
            .all(|probe| !matches!(probe, CapabilityProbeSpec::CommandHelp { .. }))
    );
    let VersionFallbackResolution::Inferred {
        state, evidence, ..
    } = VersionFallbackMap.resolve_bitbake(
        &capability,
        Some("2.8.1"),
        &[direct(CapabilityProbeStatus::Inconclusive)],
    )
    else {
        panic!("expected inferred adapter");
    };
    assert!(matches!(
        state,
        CapabilityState::AvailableWithLimitations { .. }
    ));
    assert_eq!(
        evidence.kind,
        CapabilityEvidenceKind::ReleaseVersionFallback
    );
}
