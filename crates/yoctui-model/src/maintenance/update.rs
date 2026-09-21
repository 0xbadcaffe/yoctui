pub fn update_maintenance(
    state: &mut MaintenanceState,
    action: MaintenanceAction,
) -> MaintenanceTransition {
    match action {
        action @ (MaintenanceAction::CycleView { .. }
            | MaintenanceAction::Select { .. }
            | MaintenanceAction::InspectCapability
            | MaintenanceAction::CapabilityLoaded { .. }
            | MaintenanceAction::CapabilityFailed { .. }
            | MaintenanceAction::IntegrationsLoaded { .. }
            | MaintenanceAction::IntegrationsFailed { .. }
            | MaintenanceAction::InspectServices
            | MaintenanceAction::ServicesLoaded { .. }
            | MaintenanceAction::ServicesFailed { .. }) => update_inspection(state, action),
        action @ (MaintenanceAction::OpenReadinessForm
            | MaintenanceAction::ConfirmReadinessToml(..)
            | MaintenanceAction::UpdateReadinessForm(..)
            | MaintenanceAction::ConfirmReadinessForm(..)) => update_readiness(state, action),
        action @ (MaintenanceAction::OpenCleanupForm
            | MaintenanceAction::ConfirmCleanupToml(..)
            | MaintenanceAction::UpdateCleanupForm(..)
            | MaintenanceAction::ConfirmCleanupForm(..)) => update_cleanup(state, action),
        action @ (MaintenanceAction::OpenPrServiceForm(..)
            | MaintenanceAction::ConfirmPrServiceToml { .. }
            | MaintenanceAction::UpdatePrServiceForm(..)
            | MaintenanceAction::ConfirmPrServiceForm(..)) => update_pr_service(state, action),
        action @ (MaintenanceAction::OpenLockedCacheForm
            | MaintenanceAction::ConfirmLockedCacheToml(..)
            | MaintenanceAction::UpdateLockedCacheForm(..)
            | MaintenanceAction::ConfirmLockedCacheForm(..)) => update_locked_cache(state, action),
        action @ (MaintenanceAction::OpenBuildHistoryForm
            | MaintenanceAction::ConfirmBuildHistoryToml(..)
            | MaintenanceAction::UpdateBuildHistoryForm(..)
            | MaintenanceAction::ConfirmBuildHistoryForm(..)) => update_build_history(state, action),
        action @ (MaintenanceAction::OpenGitArchiveForm
            | MaintenanceAction::ConfirmGitArchiveToml(..)
            | MaintenanceAction::UpdateGitArchiveForm(..)
            | MaintenanceAction::ConfirmGitArchiveForm(..)) => update_git_archive(state, action),
        action @ (MaintenanceAction::BeginOperation(..)
            | MaintenanceAction::UpdateCleanupPhrase { .. }
            | MaintenanceAction::ConfirmCleanupPhrase { .. }
            | MaintenanceAction::ConfirmOperation(..)
            | MaintenanceAction::ConfirmNetworkPush(..)
            | MaintenanceAction::CancelDialog) => update_operation_confirmation(state, action),
        action @ (MaintenanceAction::SessionRunning { .. }
            | MaintenanceAction::SessionOutput { .. }
            | MaintenanceAction::CompleteSession { .. }
            | MaintenanceAction::FailSession { .. }
            | MaintenanceAction::TimeoutSession { .. }
            | MaintenanceAction::LoseSession { .. }
            | MaintenanceAction::BeginCancellation
            | MaintenanceAction::ConfirmCancellation(..)
            | MaintenanceAction::RejectCancellation { .. }
            | MaintenanceAction::CancelSession { .. }
            | MaintenanceAction::SelectEvidence(..)
            | MaintenanceAction::OpenSelectedEvidence
            | MaintenanceAction::OpenSignatures
            | MaintenanceAction::OpenSecurity
            | MaintenanceAction::OpenQa
            | MaintenanceAction::OpenRecipes) => update_session_lifecycle(state, action),
    }
}
