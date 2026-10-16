use super::*;

fn editor() -> yoctui_model::RecipeEditor {
    let mut app = App::new(20, 4096);
    update(
        &mut app,
        Action::OpenRecipeEditor {
            recipe: "test".into(),
            root: "/tmp/editor workspace".into(),
            files: vec!["a file.txt".into()],
        },
    );
    match app.active_dialog().unwrap() {
        Dialog::RecipeEditor(editor) => editor.clone(),
        _ => panic!("source editor"),
    }
}

#[test]
fn preferred_editor_request_keeps_paths_and_custom_arguments_as_literal_argv() {
    let request = preferred_editor_request(
        "/bin/echo 'argument with spaces' '$(not-executed)'",
        &editor(),
    )
    .unwrap();
    assert!(request.program.is_absolute());
    assert_eq!(
        request.arguments,
        vec![
            "argument with spaces",
            "$(not-executed)",
            "/tmp/editor workspace/a file.txt"
        ]
    );
    assert_eq!(request.cwd, Path::new("/tmp/editor workspace"));
    assert!(matches!(
        request.completion,
        Some(yoctui_model::TerminalCompletion::ReturnEditor { .. })
    ));
    assert!(preferred_editor_request("yoctui-no-such-editor-874981", &editor()).is_err());
    assert!(preferred_editor_request("'unterminated", &editor()).is_err());
}

#[test]
fn full_screen_editor_command_supports_arguments_and_filename_with_spaces() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("file with spaces.txt");
    let status = run_editor_process(std::ffi::OsStr::new("/usr/bin/touch --"), &path).unwrap();
    assert!(status.success());
    assert!(path.is_file());
}
