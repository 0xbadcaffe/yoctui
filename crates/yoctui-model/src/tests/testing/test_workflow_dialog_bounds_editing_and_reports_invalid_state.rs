use super::*;

#[test]
fn test_workflow_dialog_bounds_editing_and_reports_invalid_state() {
    let mut dialog = TestLaunchDialog::new(TestLaunchDraft::new(
        TestFamily::OeSelftest,
        "qemux86-64".into(),
        "poky".into(),
        "core-image-minimal".into(),
    ));
    dialog.activate();
    dialog.select(1);
    dialog.activate();
    for _ in 0..(MAX_TEST_SELECTOR_BYTES + 10) {
        dialog.append('x');
    }
    assert_eq!(dialog.draft.selector.len(), MAX_TEST_SELECTOR_BYTES);
    dialog.finish_edit();
    dialog.select(1);
    dialog.activate();
    dialog.parallelism_input.clear();
    for character in "999".chars() {
        dialog.append(character);
    }
    dialog.finish_edit();
    assert!(dialog.editing);
    assert!(dialog.validation_error.is_some());
}
