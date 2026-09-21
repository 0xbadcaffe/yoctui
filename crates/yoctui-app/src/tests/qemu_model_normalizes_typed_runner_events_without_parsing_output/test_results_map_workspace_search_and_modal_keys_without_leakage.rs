use super::*;

#[test]
fn test_results_map_workspace_search_and_modal_keys_without_leakage() {
    assert_eq!(
        test_results_workspace_action(false, false, Input::Tab),
        Some(Action::CycleTestView)
    );
    assert_eq!(
        test_results_workspace_action(false, false, Input::Enter),
        Some(Action::DrillIntoSelectedTestResult)
    );
    assert_eq!(
        test_results_workspace_action(false, true, Input::Down),
        Some(Action::SelectTestCase { delta: 1 })
    );
    assert_eq!(
        test_results_workspace_action(false, true, Input::Esc),
        Some(Action::LeaveTestResultCases)
    );
    assert_eq!(
        test_results_workspace_action(false, false, Input::Char('I')),
        Some(Action::BeginTestResultImport)
    );
    assert_eq!(
        test_results_workspace_action(false, false, Input::Char('c')),
        Some(Action::BeginTestComparison)
    );
    assert_eq!(
        test_results_workspace_action(false, false, Input::Char('J')),
        Some(Action::BeginTestJunitExport)
    );
    assert_eq!(
        test_results_workspace_action(true, false, Input::Char('x')),
        Some(Action::AppendTestResultQuery('x')),
        "search input must not leak to the Testing workspace"
    );
    assert_eq!(
        test_result_import_dialog_action(Input::Char('/')),
        Some(Action::AppendTestResultImport('/'))
    );
    assert_eq!(
        test_result_import_dialog_action(Input::Enter),
        Some(Action::ConfirmTestResultImport)
    );
    assert_eq!(
        test_comparison_dialog_action(Input::Right),
        Some(Action::CycleTestComparisonField)
    );
    assert_eq!(
        test_comparison_dialog_action(Input::Char('p')),
        Some(Action::PreviewTestComparison)
    );
    assert_eq!(
        test_comparison_confirmation_action(Input::Enter),
        Some(Action::ConfirmTestComparison)
    );
    assert_eq!(
        test_junit_dialog_action(Input::Char('x')),
        Some(Action::AppendTestJunitDestination('x'))
    );
    assert_eq!(
        test_junit_dialog_action(Input::Enter),
        Some(Action::PreviewTestJunitExport)
    );
    assert_eq!(
        test_junit_confirmation_action(Input::Esc),
        Some(Action::CancelTestJunitExportPreview)
    );
    assert_eq!(
        test_comparison_workspace_action(Input::Char('l')),
        Some(Action::OpenSelectedTestTransitionLog)
    );
}
