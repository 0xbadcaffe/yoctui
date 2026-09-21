use super::*;

#[test]
fn ux_textarea_search_replace_undo_redo_remain_utf8_safe() {
    let mut editor = TextAreaState::new("café café\nCAFÉ".into());
    editor.search("café", true).unwrap();
    assert_eq!(editor.search_state().matches.len(), 2);
    assert!(editor.replace_selected_match("茶").unwrap());
    assert_eq!(editor.text, "茶 café\nCAFÉ");
    assert!(editor.undo());
    assert_eq!(editor.text, "café café\nCAFÉ");
    assert!(editor.redo());
    assert_eq!(editor.text, "茶 café\nCAFÉ");
    assert!(editor.text.is_char_boundary(editor.cursor));
}
