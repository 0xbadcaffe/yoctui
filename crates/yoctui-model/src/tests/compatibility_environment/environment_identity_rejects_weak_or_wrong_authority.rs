use super::*;

#[test]
fn environment_identity_rejects_weak_or_wrong_authority() {
    let mut identity = full_identity();
    identity.bitbake_version =
        AuthoritativeValue::detected("2.8.1".into(), IdentityAuthority::ReleaseMetadata);
    assert!(matches!(
        identity.normalize(),
        Err(EnvironmentIdentityError::InvalidAuthority {
            field: "bitbake_version",
            ..
        })
    ));
}
