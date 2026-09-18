use super::*;

#[test]
fn recipe_navigation_terminal_keys_and_missing_paths_are_typed() {
    for (key, expected) in [
        ('e', Action::OpenSelectedRecipeProvider),
        ('o', Action::BeginSelectedRecipeTaskLog),
        ('p', Action::BeginSelectedRecipePatchReview),
        ('d', Action::BeginSelectedRecipeDevtoolModify),
        ('u', Action::BeginSelectedRecipeDevtoolUpdateRecipe),
        ('F', Action::BeginSelectedRecipeDevtoolFinish),
        ('P', Action::BeginSelectedRecipeDevtoolDeploy),
        ('D', Action::BeginSelectedRecipeDevtoolReset),
    ] {
        let input = input_from_key(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE)).unwrap();
        assert_eq!(
            yoctui_app::recipes_workspace_action(false, input),
            Some(expected)
        );
    }
    let missing = std::env::temp_dir().join(format!(
        "yoctui-recipe-navigation-missing-{}",
        std::process::id()
    ));
    let _ = fs::remove_file(&missing);
    let error = editor_path_error(&missing).unwrap();
    assert!(error.contains("no longer exists"), "{error}");

    let missing_editor = std::ffi::OsStr::new("yoctui-editor-that-does-not-exist");
    assert_eq!(
        run_editor_process(missing_editor, Path::new("/tmp"))
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::NotFound
    );
    let status = ProcessCommand::new("/bin/sh")
        .args(["-c", "exit 7"])
        .status()
        .unwrap();
    let error = editor_exit_error(status, "/tmp/recipe.bb").unwrap();
    assert!(error.contains("exit status: 7"), "{error}");
}
