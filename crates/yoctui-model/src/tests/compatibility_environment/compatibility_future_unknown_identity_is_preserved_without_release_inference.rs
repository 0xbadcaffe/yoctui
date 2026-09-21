use super::*;

#[test]
fn compatibility_future_unknown_identity_is_preserved_without_release_inference() {
    let identity = YoctoEnvironmentIdentity {
        bitbake_version: AuthoritativeValue::detected(
            "99.0.0".into(),
            IdentityAuthority::BitBakeVersionProbe,
        ),
        oe_core: AuthoritativeValue::detected(
            ReleaseIdentity {
                name: Some("future-series".into()),
                version: None,
            },
            IdentityAuthority::ReleaseMetadata,
        ),
        ..YoctoEnvironmentIdentity::default()
    }
    .normalize()
    .unwrap();
    assert_eq!(
        identity.bitbake_version.value().map(String::as_str),
        Some("99.0.0")
    );
    assert_eq!(
        identity.oe_core.value().unwrap().name.as_deref(),
        Some("future-series")
    );
    assert_eq!(identity.poky, AuthoritativeValue::Unknown);
}
