use super::*;

#[test]
fn popup_editor_selects_native_toml_boolean_and_integer_values() {
    let mut editor = PopupEditor::new("enabled = true\njobs = 12 # bounded\n".into());
    editor.select_toml_value("enabled").unwrap();
    assert_eq!(editor.selected_text(), Some("true"));
    editor.cursor = editor.text.find("jobs").unwrap();
    editor.select_toml_value_at_cursor().unwrap();
    assert_eq!(editor.selected_text(), Some("12"));
}
