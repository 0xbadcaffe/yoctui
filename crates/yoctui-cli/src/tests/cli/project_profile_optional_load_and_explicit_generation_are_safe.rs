use super::*;

#[test]
fn project_profile_optional_load_and_explicit_generation_are_safe() {
    let root = std::env::temp_dir().join(format!("yoctui-project-profile-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir(&root).unwrap();
    assert_eq!(load_project_profile(&root).unwrap(), None);

    let profile = project_profile_fixture();
    generate_project_profile(&root, &profile, false).unwrap();
    assert_eq!(load_project_profile(&root).unwrap(), Some(profile.clone()));
    assert!(generate_project_profile(&root, &profile, false).is_err());

    let mut replacement = profile;
    replacement.favorites.recipes.push("busybox".into());
    generate_project_profile(&root, &replacement, true).unwrap();
    assert_eq!(load_project_profile(&root).unwrap(), Some(replacement));
    fs::remove_dir_all(root).unwrap();
}
