use super::*;

#[test]
fn environment_identity_preserves_unknown_for_every_partial_field() {
    let identity = YoctoEnvironmentIdentity {
        machine: AuthoritativeValue::detected(
            "qemuarm64".into(),
            IdentityAuthority::BitBakeDatastore,
        ),
        ..YoctoEnvironmentIdentity::default()
    }
    .normalize()
    .unwrap();
    assert_eq!(
        identity.machine.value().map(String::as_str),
        Some("qemuarm64")
    );
    assert_eq!(identity.bitbake_version, AuthoritativeValue::Unknown);
    assert_eq!(identity.poky, AuthoritativeValue::Unknown);
    assert_eq!(identity.available_tools, AuthoritativeValue::Unknown);
}
