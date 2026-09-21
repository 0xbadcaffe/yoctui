use super::*;

#[test]
fn raw_argv_app_validation_returns_native_elements_and_replaces_stale_results() {
    let mut editor = yoctui_model::RawArgvEditor::new("--flag 'two words'").unwrap();
    assert_eq!(
        validate_raw_argv_editor(&mut editor).unwrap(),
        ["--flag", "two words"]
    );

    editor.replace_input("left|right").unwrap();
    assert!(validate_raw_argv_editor(&mut editor).is_err());
    assert!(editor.validated.is_none());

    editor.replace_input("--next escaped\\ value").unwrap();
    assert_eq!(
        validate_raw_argv_editor(&mut editor).unwrap(),
        ["--next", "escaped value"]
    );
}
