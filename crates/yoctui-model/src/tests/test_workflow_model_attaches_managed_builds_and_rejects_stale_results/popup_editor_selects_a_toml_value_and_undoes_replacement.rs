use super::*;

#[test]
fn popup_editor_selects_a_toml_value_and_undoes_replacement() {
    let mut editor = PopupEditor::new("path = \"/old\"\nmode = \"safe\"\n".into());
    editor.select_toml_value("path").unwrap();
    assert_eq!(editor.selected_text(), Some("/old"));
    editor.insert("/new");
    assert_eq!(editor.text, "path = \"/new\"\nmode = \"safe\"\n");
    assert!(editor.undo());
    assert_eq!(editor.text, "path = \"/old\"\nmode = \"safe\"\n");
    assert!(!editor.undo());
}
