//! State transitions beginning with FailTestSession.
use super::*;

mod fail_test_session_to_test_results_cancelled;
mod test_comparison_loaded_to_test_comparison_lost;
mod test_results_timed_out_to_confirm_test_comparison;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match &action {
        Action::FailTestSession { .. }
        | Action::TimeoutTestSession { .. }
        | Action::LoseTestSession { .. }
        | Action::BeginActiveTestSessionCancellation
        | Action::ConfirmTestSessionCancellation
        | Action::CancelTestSessionCancellation
        | Action::RejectTestSessionCancellation { .. }
        | Action::CancelTestSession { .. }
        | Action::InspectResultToolCapability
        | Action::ResultToolCapabilityLoaded(..)
        | Action::CycleTestView
        | Action::SelectTestView(..)
        | Action::BeginTestResultImport
        | Action::ToggleTestResultImportTomlEditor
        | Action::AppendTestResultImportTomlEditor(..)
        | Action::BackspaceTestResultImportTomlEditor
        | Action::AppendTestResultImport(..)
        | Action::BackspaceTestResultImport
        | Action::ConfirmTestResultImport
        | Action::CancelTestResultImport
        | Action::RefreshTestResults
        | Action::TestResultsLoaded { .. }
        | Action::TestResultsFailed { .. }
        | Action::TestResultsCancelled { .. } => {
            fail_test_session_to_test_results_cancelled::reduce_actions(app, action)
        }
        Action::TestResultsTimedOut { .. }
        | Action::TestResultsLost { .. }
        | Action::SelectTestResult { .. }
        | Action::BeginTestResultSearch
        | Action::AppendTestResultQuery(..)
        | Action::BackspaceTestResultQuery
        | Action::ClearTestResultQuery
        | Action::FinishTestResultSearch
        | Action::OpenSelectedTestResult
        | Action::DrillIntoSelectedTestResult
        | Action::LeaveTestResultCases
        | Action::SelectTestCase { .. }
        | Action::OpenSelectedTestCaseLog
        | Action::BeginTestComparison
        | Action::ToggleTestComparisonTomlEditor
        | Action::AppendTestComparisonTomlEditor(..)
        | Action::BackspaceTestComparisonTomlEditor
        | Action::SelectTestComparisonChoice { .. }
        | Action::CycleTestComparisonField
        | Action::ActivateTestComparisonChoice
        | Action::PreviewTestComparison
        | Action::CancelTestComparison
        | Action::CancelTestComparisonPreview
        | Action::ConfirmTestComparison => {
            test_results_timed_out_to_confirm_test_comparison::reduce_actions(app, action)
        }
        Action::TestComparisonLoaded { .. }
        | Action::TestComparisonFailed { .. }
        | Action::TestComparisonCancelled { .. }
        | Action::TestComparisonTimedOut { .. }
        | Action::TestComparisonLost { .. } => {
            test_comparison_loaded_to_test_comparison_lost::reduce_actions(app, action)
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
}
