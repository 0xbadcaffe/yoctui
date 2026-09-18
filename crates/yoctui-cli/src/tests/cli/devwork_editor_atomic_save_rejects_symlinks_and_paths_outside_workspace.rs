use super::*;

#[cfg(unix)]
#[test]
fn devwork_editor_atomic_save_rejects_symlinks_and_paths_outside_workspace() {
    use std::os::unix::fs::symlink;
    let directory = DevworkTempDir::new();
    let outside = directory
        .0
        .parent()
        .unwrap()
        .join(format!("yoctui-devwork-outside-{}", std::process::id()));
    fs::write(&outside, "outside\n").unwrap();
    let link = directory.0.join("linked.c");
    symlink(&outside, &link).unwrap();
    let error = write_recipe_editor_file_atomically(
        &directory.0,
        &link,
        "replace\n",
        TextAreaRevision::of("outside\n"),
    )
    .unwrap_err();
    assert!(error.to_string().contains("symlink"));
    assert_eq!(fs::read_to_string(&outside).unwrap(), "outside\n");
    fs::remove_file(&outside).unwrap();
}
