use super::*;

#[test]
fn ux_textarea_page_word_and_line_motion_use_character_columns() {
    let mut editor = TextAreaState::new("one two\n猫犬\nlast line\nend".into());
    editor.layout_mut().viewport_rows = 2;
    editor.move_cursor(TextAreaMotion::DocumentStart);
    editor.move_cursor(TextAreaMotion::WordRight);
    assert_eq!(editor.position(), TextAreaPosition { line: 0, column: 4 });
    editor.move_cursor(TextAreaMotion::PageDown);
    assert_eq!(editor.position(), TextAreaPosition { line: 2, column: 4 });
    editor.move_cursor(TextAreaMotion::LineEnd);
    assert_eq!(editor.position().column, 9);
}
