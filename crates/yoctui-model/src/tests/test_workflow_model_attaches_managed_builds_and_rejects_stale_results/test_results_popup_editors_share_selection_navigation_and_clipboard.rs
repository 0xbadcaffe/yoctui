use super::*;

#[test]
fn test_results_popup_editors_share_selection_navigation_and_clipboard() {
    let mut app = test_workflow_app();
    let _ = update(&mut app, Action::BeginTestResultImport);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TestResultImportTomlEditor { editor, .. })
            if editor.selected_text() == Some("")
    ));
    assert!(matches!(
        update(
            &mut app,
            Action::EditActivePopup(PopupEditorCommand::Copy)
        ),
        Some(Effect::CopyToClipboard(value)) if value.is_empty()
    ));

    app.dialogs.clear();
    let baseline = test_results_record("baseline", "base", &[("case", TestCaseOutcome::Passed)]);
    let candidate = test_results_record(
        "candidate",
        "candidate",
        &[("case", TestCaseOutcome::Failed)],
    );
    load_test_results(&mut app, vec![baseline, candidate], Vec::new());
    let _ = update(&mut app, Action::BeginTestComparison);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::TestComparisonTomlEditor { editor, .. })
            if editor.selected_text().is_some_and(|value| value.starts_with("/build/results/"))
    ));
    assert!(matches!(
        update(
            &mut app,
            Action::EditActivePopup(PopupEditorCommand::Copy)
        ),
        Some(Effect::CopyToClipboard(value)) if value.starts_with("/build/results/")
    ));
}
