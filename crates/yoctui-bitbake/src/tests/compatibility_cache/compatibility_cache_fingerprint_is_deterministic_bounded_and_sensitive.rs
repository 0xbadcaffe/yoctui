use super::*;

#[test]
fn compatibility_cache_fingerprint_is_deterministic_bounded_and_sensitive() {
    let base_environment = environment("/poky/build", "2.8.1");
    let first = material("poky-one", "daemon-one")
        .key(base_environment.clone())
        .unwrap();
    let second = material("poky-one", "daemon-one")
        .key(base_environment.clone())
        .unwrap();
    assert_eq!(first, second);
    let mut changed = material("poky-one", "daemon-one");
    changed.initialized_environment = b"PATH=/different";
    assert_ne!(first, changed.key(base_environment).unwrap());

    let oversized = vec![0; MAX_FINGERPRINT_MATERIAL_BYTES + 1];
    let invalid = CapabilityFingerprintMaterial {
        workspace_identity: "poky-one",
        initialized_environment: &oversized,
        layer_configuration: b"layers",
        build_configuration: b"build",
        daemon_workspace_identity: "daemon-one",
    };
    assert_eq!(
        invalid.key(environment("/poky/build", "2.8.1")),
        Err(CapabilityCacheError::OversizedFingerprintMaterial)
    );
}
