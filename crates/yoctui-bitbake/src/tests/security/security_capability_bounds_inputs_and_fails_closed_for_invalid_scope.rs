use super::*;

#[test]
fn security_capability_bounds_inputs_and_fails_closed_for_invalid_scope() {
    let (_directory, mut input) = fixture(&[]);
    input.path_directories = (0..=MAX_SECURITY_PATH_DIRECTORIES)
        .map(|index| PathBuf::from(format!("/missing/{index}")))
        .collect();
    assert_eq!(
        SecurityCapabilityInspector::new(input).inspect(),
        Err(SecurityCapabilityError::TooManyInputs)
    );

    let (_directory, mut input) = fixture(&[]);
    input.scope = SecurityScope::Image {
        target: "../bad".into(),
        machine: "qemu".into(),
        distro: "poky".into(),
    };
    assert_eq!(
        SecurityCapabilityInspector::new(input).inspect(),
        Err(SecurityCapabilityError::InvalidScope)
    );
}
