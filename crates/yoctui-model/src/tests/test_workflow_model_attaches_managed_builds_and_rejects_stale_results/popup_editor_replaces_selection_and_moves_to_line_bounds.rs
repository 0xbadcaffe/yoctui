use super::*;

#[test]
fn popup_editor_replaces_selection_and_moves_to_line_bounds() {
    let mut editor = PopupEditor::new("path = \"old\"\nnext = \"value\"".into());
    editor.select_range(8, 11);
    assert_eq!(editor.selected_text(), Some("old"));
    let replacement = "/test/path/poky";
    editor.insert(replacement);
    assert!(editor.text.contains(replacement));
    editor.end();
    assert_eq!(editor.cursor, editor.text.find('\n').unwrap());
    editor.home();
    assert_eq!(editor.cursor, 0);
}
