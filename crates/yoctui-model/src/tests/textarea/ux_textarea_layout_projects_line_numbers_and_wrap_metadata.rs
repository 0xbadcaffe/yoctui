use super::*;

#[test]
fn ux_textarea_layout_projects_line_numbers_and_wrap_metadata() {
    let mut editor = TextAreaState::new("abcdef\n猫犬".into());
    editor.layout_mut().wrap_width = Some(3);
    editor.layout_mut().line_numbers = true;
    let lines = editor.visual_lines(0, 10);
    assert_eq!(lines.len(), 3);
    assert_eq!((lines[0].source_line, lines[0].continuation), (0, false));
    assert_eq!((lines[1].source_line, lines[1].continuation), (0, true));
    assert_eq!(&editor.text[lines[2].start..lines[2].end], "猫犬");
}
