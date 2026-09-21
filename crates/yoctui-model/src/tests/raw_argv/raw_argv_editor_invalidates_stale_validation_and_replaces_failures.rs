use super::*;

#[test]
fn raw_argv_editor_invalidates_stale_validation_and_replaces_failures() {
    let mut editor = RawArgvEditor::new("--flag value").unwrap();
    assert_eq!(editor.validate().unwrap().as_slice(), ["--flag", "value"]);
    editor.replace_input("'unterminated").unwrap();
    assert!(editor.validate().is_err());
    assert!(editor.validated.is_none());
    assert!(editor.validation_error.is_some());

    editor.replace_input("--next 'two words'").unwrap();
    assert!(editor.validation_error.is_none());
    assert_eq!(
        editor.validate().unwrap().as_slice(),
        ["--next", "two words"]
    );

    editor.apply(PopupEditorCommand::ToggleInsert).unwrap();
    editor.apply(PopupEditorCommand::Insert('x')).unwrap();
    assert!(editor.validated.is_none());
}
