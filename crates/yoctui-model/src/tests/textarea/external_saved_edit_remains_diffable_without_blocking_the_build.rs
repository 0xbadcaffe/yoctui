use super::*;

#[test]
fn external_saved_edit_remains_diffable_without_blocking_the_build() {
    let mut editor = TextAreaState::new("SUMMARY = \"before\"\n".into());
    editor.accept_external_edit("SUMMARY = \"after\"\n".into());

    assert!(!editor.is_modified());
    let diff = editor.preview_diff().clone();
    assert!(diff.lines.iter().any(|line| line.text.contains("before")));
    assert!(diff.lines.iter().any(|line| line.text.contains("after")));
}
