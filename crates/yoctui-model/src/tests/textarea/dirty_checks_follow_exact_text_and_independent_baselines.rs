use super::*;

#[test]
fn dirty_checks_follow_exact_text_and_independent_baselines() {
    let original = "VALUE = \"猫\"\n";
    let mut editor = TextAreaState::new(original.into());
    assert!(!editor.is_modified());
    assert!(!editor.has_visual_diff());

    // Direct public-buffer mutations must not depend on a cached edit flag,
    // including equal-byte-length UTF-8 replacements and restoration.
    editor.text = "VALUE = \"犬\"\n".into();
    assert!(editor.is_modified());
    assert!(editor.has_visual_diff());
    assert_eq!(editor.base_revision(), TextAreaRevision::of(original));
    editor.text = original.into();
    assert!(!editor.is_modified());
    assert!(!editor.has_visual_diff());

    editor.advanced.diff_base_text = "different preview baseline".into();
    assert!(!editor.is_modified());
    assert!(editor.has_visual_diff());
    editor.text.clear();
    assert!(editor.is_modified());
    assert!(editor.has_visual_diff());
}
