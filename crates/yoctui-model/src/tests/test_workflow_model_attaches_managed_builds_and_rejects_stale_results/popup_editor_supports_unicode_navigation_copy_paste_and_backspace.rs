use super::*;

#[test]
fn popup_editor_supports_unicode_navigation_copy_paste_and_backspace() {
    let mut editor = PopupEditor::new("path = \"hé\"\n".into());
    editor.select_toml_value("path").unwrap();
    assert_eq!(editor.copy_selection_or_line(), "hé");
    editor.editing = true;
    editor.insert("x");
    editor.paste();
    assert_eq!(editor.text, "path = \"xhé\"\n");
    editor.left();
    editor.backspace();
    assert_eq!(editor.text, "path = \"xé\"\n");
    editor.home();
    assert_eq!(editor.cursor, 0);
    editor.end();
    assert_eq!(editor.cursor, "path = \"xé\"".len());
}
