use super::*;

#[test]
fn raw_favorite_validates_version_name_defaults_and_excludes_runtime_authority_by_shape() {
    let catalog = catalog(1);
    let favorite = favorite_fixture(&catalog, "build.target", 0);
    favorite.validate().unwrap();
    assert_eq!(favorite.command, command("build.target"));
    assert_eq!(favorite.parameter_defaults.len(), 1);
    assert_eq!(favorite.additional_arguments.as_slice().len(), 3);

    let mut future = favorite.clone();
    future.schema_version += 1;
    assert!(matches!(
        future.validate(),
        Err(RawFavoriteError::UnsupportedSchema(_))
    ));
    let mut unnamed = favorite.clone();
    unnamed.name.clear();
    assert_eq!(unnamed.validate(), Err(RawFavoriteError::InvalidName));
    let mut invalid = favorite;
    invalid.parameter_defaults.insert(
        parameter("target"),
        RawParameterValue::Target("bad;target".into()),
    );
    assert!(matches!(
        invalid.validate(),
        Err(RawFavoriteError::InvalidDefault(_))
    ));
}
