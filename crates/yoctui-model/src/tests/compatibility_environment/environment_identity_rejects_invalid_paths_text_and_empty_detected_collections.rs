use super::*;

#[test]
fn environment_identity_rejects_invalid_paths_text_and_empty_detected_collections() {
    let mut relative = full_identity();
    relative.build_directory = AuthoritativeValue::detected(
        "relative/build".into(),
        IdentityAuthority::InitializedEnvironment,
    );
    assert_eq!(
        relative.normalize(),
        Err(EnvironmentIdentityError::InvalidField("build_directory"))
    );

    let mut control = full_identity();
    control.machine =
        AuthoritativeValue::detected("qemu\narm".into(), IdentityAuthority::BitBakeDatastore);
    assert_eq!(
        control.normalize(),
        Err(EnvironmentIdentityError::InvalidField("machine"))
    );

    let mut empty = full_identity();
    empty.available_tools =
        AuthoritativeValue::detected(Vec::new(), IdentityAuthority::ExecutableProbe);
    assert_eq!(
        empty.normalize(),
        Err(EnvironmentIdentityError::InvalidField("available_tools"))
    );
}
