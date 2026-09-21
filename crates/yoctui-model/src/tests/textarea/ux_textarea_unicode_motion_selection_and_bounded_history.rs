use super::*;

#[test]
fn ux_textarea_unicode_motion_selection_and_bounded_history() {
    let mut editor = TextAreaState::new("αβ\n猫 dog".into());
    editor.move_cursor(TextAreaMotion::DocumentStart);
    editor.set_mode(TextAreaMode::Visual);
    editor.move_cursor(TextAreaMotion::Right);
    editor.move_cursor(TextAreaMotion::Right);
    assert_eq!(editor.selected_text(), Some("αβ"));
    editor.set_mode(TextAreaMode::Insert);
    editor.try_insert("🙂").unwrap();
    for _ in 0..(TEXTAREA_MAX_HISTORY + 20) {
        editor.try_insert("x").unwrap();
    }
    assert_eq!(editor.history_lengths().0, TEXTAREA_MAX_HISTORY);
    assert!(editor.text.is_char_boundary(editor.cursor));
}
