use super::*;

#[test]
fn compatibility_version_unknown_malformed_and_future_versions_fail_closed() {
    let map = VersionFallbackMap;
    let capability = entry(CapabilityId::BitBakeBuild);
    for version in [
        None,
        Some("future"),
        Some("1.45"),
        Some("2.19"),
        Some("3.0"),
    ] {
        let resolution = map.resolve_bitbake(&capability, version, &[]);
        assert!(
            matches!(resolution, VersionFallbackResolution::Unknown { .. }),
            "{version:?}"
        );
    }
}
