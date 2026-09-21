use super::*;

#[test]
fn project_profile_rejects_schema_duplicates_and_unknown_presets() {
    let mut value = profile();
    value.schema_version = 2;
    assert_eq!(
        value.validate(),
        Err(ProjectProfileError::UnsupportedSchema(2))
    );
    let mut value = profile();
    value.favorites.images.push("core-image-minimal".into());
    assert!(matches!(
        value.validate(),
        Err(ProjectProfileError::InvalidField { .. })
    ));
    let mut value = profile();
    value.workflows[0].steps[1] = ProjectWorkflowStep::UseBuildPreset {
        preset: "missing".into(),
    };
    assert_eq!(
        value.validate(),
        Err(ProjectProfileError::UnknownPreset("missing".into()))
    );
}
