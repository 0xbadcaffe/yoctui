use super::*;

#[test]
fn capability_snapshot_keeps_exact_environment_association() {
    let first = CapabilitySnapshot {
        generation: 1,
        environment: environment(),
        capabilities: Vec::new(),
    }
    .normalize()
    .unwrap();
    let mut other_environment = environment();
    other_environment.build_directory = AuthoritativeValue::detected(
        "/other/build".into(),
        IdentityAuthority::InitializedEnvironment,
    );
    let second = CapabilitySnapshot {
        generation: 2,
        environment: other_environment,
        capabilities: Vec::new(),
    }
    .normalize()
    .unwrap();
    assert_ne!(first.environment, second.environment);
    assert_eq!(
        first.environment.build_directory.value(),
        Some(&PathBuf::from("/work/build"))
    );
    assert_eq!(
        second.environment.build_directory.value(),
        Some(&PathBuf::from("/other/build"))
    );
}
