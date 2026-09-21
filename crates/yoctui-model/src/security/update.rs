pub fn update_security(state: &mut SecurityState, action: SecurityAction) -> SecurityTransition {
    match action {
        action @ (SecurityAction::InspectCapability
            | SecurityAction::CapabilityLoaded(..)
            | SecurityAction::CapabilityFailed(..)
            | SecurityAction::CycleView
            | SecurityAction::CycleScope
            | SecurityAction::SetScope(..)
            | SecurityAction::BeginCveCheck
            | SecurityAction::BeginSbomGeneration
            | SecurityAction::BeginPackageMap
            | SecurityAction::ConfirmOperation(..)) => update_capability(state, action),
        action @ (SecurityAction::AttachBackgroundJob { .. }
            | SecurityAction::SessionRunning(..)
            | SecurityAction::SessionOutput { .. }
            | SecurityAction::CompleteSession { .. }
            | SecurityAction::FailSession { .. }
            | SecurityAction::LoseSession { .. }
            | SecurityAction::TimeoutSession { .. }
            | SecurityAction::BeginCancellation
            | SecurityAction::ConfirmCancellation(..)
            | SecurityAction::RejectCancellation { .. }
            | SecurityAction::CancelSession { .. }) => update_session(state, action),
        action @ (SecurityAction::BeginImport
            | SecurityAction::UpdateImport(..)
            | SecurityAction::ConfirmImport(..)
            | SecurityAction::CancelDialog) => update_imports(state, action),
        action @ (SecurityAction::RefreshReports
            | SecurityAction::ReportsLoaded { .. }
            | SecurityAction::ReportsFailed { .. }
            | SecurityAction::ReportsCancelled(..)
            | SecurityAction::ReportsTimedOut(..)
            | SecurityAction::ReportsLost { .. }) => update_reports(state, action),
        action @ (SecurityAction::SelectReport(..)
            | SecurityAction::SelectFinding(..)
            | SecurityAction::SelectComponent(..)
            | SecurityAction::Drill
            | SecurityAction::LeaveDrill
            | SecurityAction::BeginSearch
            | SecurityAction::AppendQuery(..)
            | SecurityAction::BackspaceQuery
            | SecurityAction::ClearQuery
            | SecurityAction::FinishSearch
            | SecurityAction::CycleCveFilter
            | SecurityAction::OpenSelectedReport
            | SecurityAction::OpenSelectedRecipe
            | SecurityAction::OpenSelectedAdvisory) => update_navigation(state, action),
    }
}
