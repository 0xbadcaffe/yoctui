pub fn update_qa(state: &mut QaState, action: QaAction) -> QaTransition {
    match action {
        action @ (QaAction::CycleView
            | QaAction::InspectCapability
            | QaAction::CapabilityLoaded(..)
            | QaAction::CapabilityPartial { .. }
            | QaAction::CapabilityFailed(..)
            | QaAction::CycleScope
            | QaAction::SelectCheck(..)
            | QaAction::BeginSelectedCheck) => update_recipe_capability(state, action),
        action @ (QaAction::ConfirmOperation(..)
            | QaAction::AttachBackgroundJob { .. }
            | QaAction::SessionRunning(..)
            | QaAction::SessionOutput { .. }
            | QaAction::CompleteSession { .. }
            | QaAction::FailSession { .. }
            | QaAction::TimeoutSession { .. }
            | QaAction::LoseSession { .. }
            | QaAction::BeginCancellation
            | QaAction::ConfirmCancellation(..)
            | QaAction::RejectCancellation { .. }
            | QaAction::CancelSession { .. }) => update_recipe_session(state, action),
        action @ (QaAction::BeginImport
            | QaAction::UpdateImport(..)
            | QaAction::ConfirmImport(..)
            | QaAction::CancelDialog) => update_imports(state, action),
        action @ (QaAction::RefreshReports
            | QaAction::ReportsLoaded { .. }
            | QaAction::ReportsFailed { .. }
            | QaAction::ReportsCancelled(..)
            | QaAction::ReportsTimedOut(..)
            | QaAction::ReportsLost { .. }) => update_reports(state, action),
        action @ (QaAction::SelectReport(..)
            | QaAction::SelectFinding(..)
            | QaAction::Drill
            | QaAction::LeaveDrill
            | QaAction::BeginSearch
            | QaAction::AppendQuery(..)
            | QaAction::BackspaceQuery
            | QaAction::ClearQuery
            | QaAction::FinishSearch
            | QaAction::CycleStatusFilter
            | QaAction::OpenSelectedReport
            | QaAction::OpenProvider
            | QaAction::OpenSelectedSource) => update_report_navigation(state, action),
        action @ (QaAction::InspectLayerCapability
            | QaAction::LayerCapabilityLoaded(..)
            | QaAction::LayerCapabilityPartial { .. }
            | QaAction::LayerCapabilityFailed(..)
            | QaAction::SelectLayer(..)
            | QaAction::BeginSelectedLayerCheck
            | QaAction::ConfirmLayerOperation(..)) => update_layer_capability(state, action),
        action @ (QaAction::LayerSessionRunning(..)
            | QaAction::LayerSessionOutput { .. }
            | QaAction::CompleteLayerSession { .. }
            | QaAction::FailLayerSession { .. }
            | QaAction::TimeoutLayerSession { .. }
            | QaAction::LoseLayerSession { .. }
            | QaAction::BeginLayerCancellation
            | QaAction::ConfirmLayerCancellation(..)
            | QaAction::RejectLayerCancellation { .. }
            | QaAction::CancelLayerSession { .. }
            | QaAction::OpenSelectedLayerRoot) => update_layer_session(state, action),
    }
}
