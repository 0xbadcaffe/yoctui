use super::*;

#[test]
fn compatibility_version_direct_evidence_overrides_static_inference() {
    let map = VersionFallbackMap;
    for (status, outcome) in [
        (
            CapabilityProbeStatus::Positive,
            CapabilityEvidenceOutcome::Positive,
        ),
        (
            CapabilityProbeStatus::Negative,
            CapabilityEvidenceOutcome::Negative,
        ),
    ] {
        assert_eq!(
            map.resolve_bitbake(
                &entry(CapabilityId::BitBakeNativeEvents),
                Some("1.52"),
                &[direct(status)],
            ),
            VersionFallbackResolution::Direct { outcome }
        );
    }
    assert!(matches!(
        map.resolve_bitbake(
            &entry(CapabilityId::BitBakeNativeEvents),
            Some("2.8"),
            &[
                direct(CapabilityProbeStatus::Positive),
                direct(CapabilityProbeStatus::Negative)
            ],
        ),
        VersionFallbackResolution::Unknown { .. }
    ));
}
