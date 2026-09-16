//! Reducer.
use super::*;

mod begin_selected_recipe_devtool_update_recipe;
mod build_cancelled;
mod cancel_sdk_publish;
mod complete_qemu_session;
mod confirm_keymap_capture;
mod confirm_wic_device_selection;
mod fail_test_session;
mod open_onboarding;
mod open_selected_package_recipe;
mod reset_pane_subfocus;
mod select_test_comparison_transition;
mod set_layer_inspector_mode;
mod terminal_append_search;
pub fn update(app: &mut App, action: Action) -> Option<Effect> {
    if modal_focus(app).is_some()
        && matches!(
            &action,
            Action::OpenOnboarding
                | Action::Open(_)
                | Action::OpenRawFavorites
                | Action::SelectNavigator { .. }
                | Action::SelectNavigatorAt { .. }
                | Action::ToggleNavigatorGroup { .. }
                | Action::CollapseNavigatorGroup
                | Action::ExpandNavigatorGroup
                | Action::ActivateNavigator
                | Action::CycleFocus { .. }
                | Action::CyclePaneSubfocus { .. }
                | Action::ResetPaneSubfocus
                | Action::TogglePaneZoom
                | Action::ScrollCurrent { .. }
                | Action::Focus(
                    FocusTarget::Navigator | FocusTarget::Workspace | FocusTarget::Inspector
                )
                | Action::OpenCommandPalette
                | Action::OpenBuildOptions
                | Action::OpenImagePicker(_)
        )
    {
        return None;
    }
    match action {
        Action::SourceGitStatusUpdated(status) => {
            app.source_git_status = status;
            None
        }
        Action::SetBackgroundActivity { activity, active } => {
            if active {
                app.background_activities.insert(activity);
            } else {
                app.background_activities.remove(&activity);
            }
            None
        }

        Action::OpenOnboarding
        | Action::DismissOnboarding
        | Action::SelectOnboarding { .. }
        | Action::ActivateOnboardingStep
        | Action::AdvanceOnboarding
        | Action::SkipOnboardingStep
        | Action::RestartOnboarding
        | Action::OnboardingPersisted
        | Action::OnboardingPersistenceFailed(..)
        | Action::ScrollCurrent { .. }
        | Action::ProjectProfileAbsent
        | Action::ProjectProfileLoaded(..)
        | Action::ProjectProfileLoadFailed(..)
        | Action::PreviewProjectProfileGeneration(..)
        | Action::ConfirmProjectProfileGeneration { .. }
        | Action::ProjectProfileGenerated(..)
        | Action::ProjectProfileGenerationFailed(..)
        | Action::SelectProjectProfileItem { .. }
        | Action::ActivateProjectProfileItem
        | Action::OpenRawFavorites
        | Action::InspectKernel
        | Action::KernelLoaded(..)
        | Action::KernelFailed(..)
        | Action::CycleKernelView
        | Action::SelectKernelFile { .. }
        | Action::LaunchKernelMenuconfig
        | Action::OpenSelectedKernelFile
        | Action::ExploreSelectedKernelRoot
        | Action::CompileSelectedKernelDts
        | Action::DecompileSelectedKernelDtb
        | Action::SelectDtcCompileOption { .. }
        | Action::AdjustDtcCompileOption { .. }
        | Action::ConfirmDtcCompileOptions
        | Action::CancelDtcCompileOptions
        | Action::InspectFirmware
        | Action::FirmwareLoaded(..)
        | Action::FirmwareFailed(..)
        | Action::CycleFirmwareView
        | Action::SelectFirmwareFile { .. }
        | Action::LaunchFirmwareMenuconfig
        | Action::OpenSelectedFirmwareFile
        | Action::ExploreSelectedFirmwareRoot
        | Action::CompileSelectedFirmwareDts
        | Action::DecompileSelectedFirmwareDtb
        | Action::ShiftOverviewView { .. }
        | Action::SelectOverviewView(..)
        | Action::Open(..)
        | Action::SelectNavigator { .. }
        | Action::SelectNavigatorAt { .. }
        | Action::ToggleNavigatorGroup { .. }
        | Action::CollapseNavigatorGroup
        | Action::ExpandNavigatorGroup
        | Action::SelectPtySession { .. }
        | Action::SelectPtyPane { .. }
        | Action::TerminalTakeControl
        | Action::TerminalReleaseControl
        | Action::TerminalCreateBuildShell
        | Action::TerminalCreateSelectedDevshell
        | Action::TerminalCreateSelectedMenuconfig
        | Action::DetachedTerminalAvailabilityDetected(..)
        | Action::SelectTerminalLaunchDestination { .. }
        | Action::ConfirmTerminalLaunch
        | Action::CancelTerminalLaunch
        | Action::TerminalEnterCopyMode
        | Action::TerminalMoveCopyRow { .. }
        | Action::TerminalCopyViewport
        | Action::TerminalBeginSearch => open_onboarding::reduce_actions(app, action),
        Action::TerminalAppendSearch(..)
        | Action::TerminalBackspaceSearch
        | Action::TerminalFinishSearch
        | Action::TerminalClearSearch
        | Action::TerminalStagePaste(..)
        | Action::TerminalConfirmPaste
        | Action::TerminalBeginRename
        | Action::TerminalAppendRename(..)
        | Action::TerminalBackspaceRename
        | Action::TerminalConfirmRename
        | Action::TerminalScroll { .. }
        | Action::TerminalBeginKill
        | Action::TerminalConfirmKill
        | Action::TerminalCancelMode
        | Action::TerminalToggleHelp
        | Action::ResizeFocusedPane { .. }
        | Action::ActivateNavigator
        | Action::Security(..)
        | Action::Qa(..)
        | Action::Maintenance(..)
        | Action::Focus(..)
        | Action::OpenCommandPalette
        | Action::OpenGlobalSearch
        | Action::SelectCommandPalette { .. }
        | Action::AppendCommandPaletteQuery(..)
        | Action::BackspaceCommandPaletteQuery
        | Action::ClearCommandPaletteQuery
        | Action::BeginGlobalContentSearch
        | Action::GlobalContentSearchLoaded { .. }
        | Action::GlobalContentSearchFailed { .. }
        | Action::ActivateCommandPalette
        | Action::CloseCommandPalette
        | Action::OpenApplicationMenu
        | Action::OpenContextMenu
        | Action::SelectMenuGroup { .. }
        | Action::SelectMenuItem { .. }
        | Action::AppendMenuPrefix(..)
        | Action::BackspaceMenuPrefix
        | Action::CloseMenu
        | Action::SelectSetting { .. }
        | Action::ChangeSelectedSetting { .. }
        | Action::ResetPreferences
        | Action::RetrySettingsPersistence
        | Action::OpenKeymapPreferences
        | Action::CloseKeymapPreferences
        | Action::SelectKeymapPreference { .. }
        | Action::BeginKeymapPreferenceSearch
        | Action::AppendKeymapPreferenceQuery(..)
        | Action::BackspaceKeymapPreferenceQuery
        | Action::ClearKeymapPreferenceQuery
        | Action::FinishKeymapPreferenceSearch
        | Action::BeginKeymapCapture
        | Action::AppendKeymapCapture(..)
        | Action::BackspaceKeymapCapture
        | Action::AppendInternalLogQuery(..)
        | Action::BackspaceInternalLogQuery
        | Action::AppendLogQuery(..)
        | Action::BackspaceLogQuery
        | Action::NextLogMatch
        | Action::PreviousLogMatch
        | Action::AppendMetadataQuery(..)
        | Action::BackspaceMetadataQuery => terminal_append_search::reduce_actions(app, action),
        Action::ConfirmKeymapCapture
        | Action::CancelKeymapCapture
        | Action::RemoveKeymapBinding
        | Action::ResetKeymapBinding
        | Action::ResetAllKeymapBindings
        | Action::ExportEffectiveKeymap
        | Action::SettingsPersisted
        | Action::SettingsPersistenceFailed(..)
        | Action::SetCompatibilityFilter(..)
        | Action::SelectCompatibilityCapability { .. }
        | Action::BeginCompatibilitySearch
        | Action::AppendCompatibilityQuery(..)
        | Action::BackspaceCompatibilityQuery
        | Action::ClearCompatibilityQuery
        | Action::FinishCompatibilitySearch
        | Action::RawMode(..)
        | Action::EditActivePopup(..)
        | Action::OpenBuildEnvironmentCloneEditor
        | Action::ToggleBuildEnvironmentCloneEditor
        | Action::AppendBuildEnvironmentCloneEditor(..)
        | Action::BackspaceBuildEnvironmentCloneEditor
        | Action::ReviewBuildEnvironmentClone
        | Action::ConfirmBuildEnvironmentClone
        | Action::CancelBuildEnvironmentClone
        | Action::EnvironmentSetup(..)
        | Action::OpenBuildEnvironmentEditor
        | Action::ToggleBuildEnvironmentEditor
        | Action::AppendBuildEnvironmentEditor(..)
        | Action::BackspaceBuildEnvironmentEditor
        | Action::ApplyBuildEnvironmentEditor
        | Action::CloseBuildEnvironmentEditor
        | Action::OpenThemePicker
        | Action::SelectTheme { .. }
        | Action::ApplySelectedTheme
        | Action::CloseThemePicker
        | Action::ConfigureBuildEnvironment(..)
        | Action::BeginBuildEnvironmentEdit
        | Action::SelectBuildEnvironmentField { .. }
        | Action::AppendBuildEnvironmentField(..)
        | Action::BackspaceBuildEnvironmentField
        | Action::FinishBuildEnvironmentEdit
        | Action::CancelBuildEnvironmentEdit
        | Action::ApplyBuildEnvironmentProfile
        | Action::BeginBuildEnvironmentVerification
        | Action::BuildEnvironmentVerified { .. }
        | Action::BuildEnvironmentVerificationFailed { .. }
        | Action::CycleFocus { .. }
        | Action::CyclePaneSubfocus { .. } => confirm_keymap_capture::reduce_actions(app, action),
        Action::ResetPaneSubfocus
        | Action::TogglePaneZoom
        | Action::OpenBuildOptions
        | Action::CloseBuildOptions
        | Action::OpenImagePicker(..)
        | Action::SelectImage { .. }
        | Action::ConfirmImagePicker
        | Action::CancelImagePicker
        | Action::BeginCurrentImageBuild
        | Action::BeginImageArtifactInventory
        | Action::RefreshImageArtifactInventory
        | Action::CancelImageArtifactOperation
        | Action::ImageArtifactInventoryLoaded { .. }
        | Action::ImageArtifactInventoryPartial { .. }
        | Action::ImageArtifactInventoryFailed { .. }
        | Action::SelectImageArtifact { .. }
        | Action::BeginImageArtifactSearch
        | Action::AppendImageArtifactQuery(..)
        | Action::BackspaceImageArtifactQuery
        | Action::ClearImageArtifactQuery
        | Action::FinishImageArtifactSearch
        | Action::BeginSelectedImageArtifactBuild
        | Action::OpenSelectedImageArtifact
        | Action::OpenSelectedImageArtifactAssociation(..)
        | Action::BeginSelectedRootfsComposition
        | Action::ShiftImagesView { .. }
        | Action::RefreshRootfsComposition
        | Action::RootfsCompositionLoaded { .. }
        | Action::RootfsCompositionPartial { .. }
        | Action::RootfsCompositionUnavailable { .. }
        | Action::RootfsCompositionFailed { .. }
        | Action::SelectRootfsGroup { .. }
        | Action::SelectRootfsPackage { .. }
        | Action::SelectRootfsEntry { .. }
        | Action::SelectRootfsSystemdService { .. }
        | Action::SelectRootfsDbusService { .. }
        | Action::SelectRootfsUdevRule { .. }
        | Action::ScrollRootfsUdevPreview { .. }
        | Action::BrowseRootfsFilesystem
        | Action::EditSelectedRootfsSystemFile
        | Action::BeginSdkBuild(..)
        | Action::ConfirmSdkBuild
        | Action::CancelSdkBuild
        | Action::BeginSdkArtifactInventory
        | Action::RefreshSdkArtifactInventory
        | Action::SdkArtifactInventoryLoaded { .. }
        | Action::SdkArtifactInventoryFailed { .. }
        | Action::SelectSdkArtifact { .. }
        | Action::BeginSdkArtifactSearch
        | Action::AppendSdkArtifactQuery(..)
        | Action::BackspaceSdkArtifactQuery
        | Action::ClearSdkArtifactQuery
        | Action::FinishSdkArtifactSearch
        | Action::OpenSelectedSdkArtifact
        | Action::SdkToolCapabilityLoaded(..)
        | Action::BeginSelectedSdkPublish
        | Action::ToggleSdkPublishTomlEditor
        | Action::AppendSdkPublishTomlEditor(..)
        | Action::BackspaceSdkPublishTomlEditor
        | Action::AppendSdkPublishDestination(..)
        | Action::BackspaceSdkPublishDestination
        | Action::PreviewSdkPublish => reset_pane_subfocus::reduce_actions(app, action),
        Action::CancelSdkPublish
        | Action::CancelSdkPublishPreview
        | Action::ConfirmSdkPublish
        | Action::BeginSdkNative
        | Action::ToggleSdkNativeTomlEditor
        | Action::AppendSdkNativeTomlEditor(..)
        | Action::BackspaceSdkNativeTomlEditor
        | Action::UpdateSdkNativeDraft(..)
        | Action::SelectSdkNativeField { .. }
        | Action::ActivateSdkNativeField
        | Action::CycleSdkNativeMode
        | Action::AppendSdkNativeField(..)
        | Action::BackspaceSdkNativeField
        | Action::FinishSdkNativeFieldEdit
        | Action::PreviewSdkNative
        | Action::CancelSdkNative
        | Action::CancelSdkNativePreview
        | Action::ConfirmSdkNative
        | Action::SdkSessionStarting { .. }
        | Action::SdkSessionRunning { .. }
        | Action::AppendSdkSessionOutput { .. }
        | Action::CompleteSdkSession { .. }
        | Action::FailSdkSession { .. }
        | Action::LoseSdkSession { .. }
        | Action::BeginActiveSdkSessionCancellation
        | Action::ConfirmSdkSessionCancellation
        | Action::CancelSdkSessionCancellation
        | Action::RejectSdkSessionCancellation { .. }
        | Action::CancelSdkSession { .. }
        | Action::InspectTestCapability
        | Action::TestCapabilityLoaded(..)
        | Action::SelectTestFamily { .. }
        | Action::BeginSelectedTestLaunch
        | Action::ToggleTestLaunchTomlEditor
        | Action::AppendTestLaunchTomlEditor(..)
        | Action::BackspaceTestLaunchTomlEditor
        | Action::UpdateTestLaunchDraft(..)
        | Action::SelectTestLaunchField { .. }
        | Action::ActivateTestLaunchField
        | Action::AppendTestLaunchField(..)
        | Action::BackspaceTestLaunchField
        | Action::FinishTestLaunchFieldEdit
        | Action::PreviewTestLaunch
        | Action::CancelTestLaunch
        | Action::CancelTestLaunchPreview
        | Action::ConfirmTestLaunch
        | Action::AttachTestBuildSession { .. }
        | Action::TestSessionStarting { .. }
        | Action::TestSessionRunning { .. }
        | Action::AppendTestSessionOutput { .. }
        | Action::CompleteTestSession { .. } => cancel_sdk_publish::reduce_actions(app, action),
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
        | Action::TestResultsCancelled { .. }
        | Action::TestResultsTimedOut { .. }
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
        | Action::ConfirmTestComparison
        | Action::TestComparisonLoaded { .. }
        | Action::TestComparisonFailed { .. }
        | Action::TestComparisonCancelled { .. }
        | Action::TestComparisonTimedOut { .. }
        | Action::TestComparisonLost { .. } => fail_test_session::reduce_actions(app, action),
        Action::SelectTestComparisonTransition { .. }
        | Action::OpenSelectedTestTransitionLog
        | Action::BeginTestJunitExport
        | Action::ToggleTestJunitTomlEditor
        | Action::AppendTestJunitTomlEditor(..)
        | Action::BackspaceTestJunitTomlEditor
        | Action::MoveTestJunitTomlEditorLeft
        | Action::MoveTestJunitTomlEditorRight
        | Action::MoveTestJunitTomlEditorUp
        | Action::MoveTestJunitTomlEditorDown
        | Action::MoveTestJunitTomlEditorHome
        | Action::MoveTestJunitTomlEditorEnd
        | Action::SelectTestJunitDestination
        | Action::CopyTestJunitTomlEditor
        | Action::PasteTestJunitTomlEditor
        | Action::AppendTestJunitDestination(..)
        | Action::BackspaceTestJunitDestination
        | Action::PreviewTestJunitExport
        | Action::CancelTestJunitExport
        | Action::TestJunitDestinationInspected { .. }
        | Action::CancelTestJunitExportPreview
        | Action::ConfirmTestJunitExport
        | Action::TestJunitExportSucceeded { .. }
        | Action::TestJunitExportFailed { .. }
        | Action::TestJunitExportCancelled { .. }
        | Action::TestJunitExportTimedOut { .. }
        | Action::TestJunitExportLost { .. }
        | Action::InspectQemuCapability
        | Action::QemuCapabilityLoaded(..)
        | Action::SshClientCapabilityDetected(..)
        | Action::BeginSelectedImageConsole
        | Action::SelectImageConsoleField { .. }
        | Action::CycleImageConsoleChoice { .. }
        | Action::AppendImageConsoleField(..)
        | Action::BackspaceImageConsoleField
        | Action::ConfirmImageConsole
        | Action::CancelImageConsole
        | Action::BeginSelectedQemuLaunch
        | Action::UpdateQemuLaunchDraft(..)
        | Action::SelectQemuLaunchField { .. }
        | Action::ActivateQemuLaunchField
        | Action::CycleQemuLaunchChoice { .. }
        | Action::AppendQemuLaunchField(..)
        | Action::BackspaceQemuLaunchField
        | Action::FinishQemuLaunchFieldEdit
        | Action::PreviewQemuLaunch
        | Action::CancelQemuLaunch
        | Action::CancelQemuLaunchPreview
        | Action::ConfirmQemuLaunchInTerminal
        | Action::ConfirmQemuLaunch
        | Action::QemuSessionStarting { .. }
        | Action::QemuSessionRunning { .. }
        | Action::AppendQemuSessionOutput { .. } => {
            select_test_comparison_transition::reduce_actions(app, action)
        }
        Action::CompleteQemuSession { .. }
        | Action::FailQemuSession { .. }
        | Action::LoseQemuSession { .. }
        | Action::BeginQemuSessionCancellation { .. }
        | Action::BeginActiveQemuSessionCancellation
        | Action::ConfirmQemuSessionCancellation
        | Action::CancelQemuSessionCancellation
        | Action::RejectQemuSessionCancellation { .. }
        | Action::CancelQemuSession { .. }
        | Action::InspectWicCapability
        | Action::WicCapabilityLoaded(..)
        | Action::BeginSelectedWicCreate
        | Action::ToggleWicCreateTomlEditor
        | Action::AppendWicCreateTomlEditor(..)
        | Action::BackspaceWicCreateTomlEditor
        | Action::SelectWicCreateField { .. }
        | Action::ActivateWicCreateField
        | Action::CycleWicCreateChoice { .. }
        | Action::AppendWicCreateField(..)
        | Action::BackspaceWicCreateField
        | Action::FinishWicCreateFieldEdit
        | Action::PreviewWicCreate
        | Action::CancelWicCreate
        | Action::CancelWicCreatePreview
        | Action::ConfirmWicCreate
        | Action::SelectWicOutput { .. }
        | Action::OpenSelectedWicOutput
        | Action::BeginActiveWicSessionCancellation
        | Action::BeginActiveImageRuntimeCancellation
        | Action::CancelWicSessionCancellation
        | Action::BeginWicOutputInventory(..)
        | Action::WicOutputInventoryLoaded { .. }
        | Action::WicOutputInventoryFailed { .. }
        | Action::BeginSelectedWicDeviceWrite
        | Action::BeginWicDeviceInventory(..)
        | Action::WicDeviceInventoryLoaded { .. }
        | Action::WicDeviceInventoryFailed { .. }
        | Action::SelectWicDevice { .. } => complete_qemu_session::reduce_actions(app, action),
        Action::ConfirmWicDeviceSelection
        | Action::CancelWicDevicePicker
        | Action::AppendWicWritePhrase(..)
        | Action::BackspaceWicWritePhrase
        | Action::PreviewWicDeviceWrite
        | Action::CancelWicWritePhrase
        | Action::ConfirmWicDeviceWrite
        | Action::CancelWicWritePreview
        | Action::StartConfirmedWicCreate(..)
        | Action::WicSessionStarting { .. }
        | Action::WicSessionRunning { .. }
        | Action::AppendWicSessionOutput { .. }
        | Action::CompleteWicSession { .. }
        | Action::FailWicSession { .. }
        | Action::LoseWicSession { .. }
        | Action::ConfirmWicSessionCancellation { .. }
        | Action::RejectWicSessionCancellation { .. }
        | Action::CancelWicSession { .. }
        | Action::BeginBuildTargetEdit
        | Action::BeginBuildTargetTask(..)
        | Action::ToggleBuildTargetEdit
        | Action::AppendBuildTarget(..)
        | Action::BackspaceBuildTarget
        | Action::CancelBuildTargetEdit
        | Action::ConfirmBuildTarget
        | Action::Start(..)
        | Action::QueueBackgroundJob(..)
        | Action::StartBackgroundJob { .. }
        | Action::RunBackgroundJob { .. }
        | Action::UpdateBackgroundJobProgress { .. }
        | Action::AppendBackgroundJobOutput { .. }
        | Action::RequestBackgroundJobCancellation { .. }
        | Action::RejectBackgroundJobCancellation { .. }
        | Action::SucceedBackgroundJob { .. }
        | Action::FailBackgroundJob { .. }
        | Action::CancelBackgroundJob { .. }
        | Action::LoseBackgroundJob { .. }
        | Action::BuildRequested { .. }
        | Action::BuildStarted
        | Action::TaskStats(..)
        | Action::ParseProgress { .. }
        | Action::TaskStarted(..)
        | Action::TaskQueued(..)
        | Action::TaskProgress { .. }
        | Action::TaskCompleted { .. }
        | Action::TaskEvents(..)
        | Action::ScrollBuildTasks { .. }
        | Action::CycleTaskStateFilter
        | Action::CycleTaskFilterField
        | Action::BeginTaskFilterEdit
        | Action::AppendTaskFilter(..)
        | Action::BackspaceTaskFilter
        | Action::FinishTaskFilterEdit
        | Action::CycleTaskDurationFilter
        | Action::Log(..)
        | Action::Logs(..)
        | Action::BuildCompleted { .. }
        | Action::BuildAuthorityLost { .. } => {
            confirm_wic_device_selection::reduce_actions(app, action)
        }
        Action::BuildCancelled { .. }
        | Action::BuildCancellationRejected(..)
        | Action::DismissBuildCompletion
        | Action::OpenBuildCompletionErrors
        | Action::SelectBuildHistory { .. }
        | Action::Cancel
        | Action::ConfirmBuildCancellation
        | Action::CancelBuildCancellation
        | Action::CycleLogWorkspaceView
        | Action::InternalLog(..)
        | Action::InternalLogIngressDropped(..)
        | Action::ToggleInternalLogFollow
        | Action::ScrollInternalLogs { .. }
        | Action::BeginInternalLogSearch
        | Action::ClearInternalLogQuery
        | Action::FinishInternalLogSearch
        | Action::CycleInternalLogLevelFilter
        | Action::CycleInternalLogTargetFilter
        | Action::ClearInternalLogs
        | Action::ExportInternalLogs
        | Action::ToggleLogFollow
        | Action::ToggleLogWrap
        | Action::CycleLogSeverity
        | Action::ScrollLogs { .. }
        | Action::BeginLogSearch
        | Action::ClearLogQuery
        | Action::FinishLogSearch
        | Action::ScrollLogsHorizontally { .. }
        | Action::CycleLogRecipeFilter
        | Action::CycleLogTaskFilter
        | Action::CycleLogBuildFilter
        | Action::CycleLogSourceFilter
        | Action::CycleLogTimeRange
        | Action::ToggleSelectedLogBookmark
        | Action::NextLogBookmark
        | Action::PreviousLogBookmark
        | Action::OpenSelectedLogSource
        | Action::CopySelectedLog
        | Action::ExportFilteredLogs
        | Action::SelectError { .. }
        | Action::JumpToSelectedError
        | Action::OpenSelectedErrorSource
        | Action::SelectRecipe { .. }
        | Action::ScrollRecipePreview { .. }
        | Action::BeginSelectedRecipeBuild
        | Action::BeginSelectedRecipeClean
        | Action::BeginSelectedRecipeMenuConfig
        | Action::BeginSelectedRecipeCleanState
        | Action::BeginSelectedRecipeDevshell
        | Action::BeginSelectedRecipeDevtoolWorkspaceShell
        | Action::BeginSelectedRecipeDevtoolEditRecipe
        | Action::BeginSelectedRecipeDiffconfig
        | Action::BeginSelectedRecipeDiffsigs
        | Action::BeginSelectedRecipeSignatures
        | Action::BeginSelectedRecipeCveCheck
        | Action::BeginSelectedRecipeSpdx
        | Action::BeginSelectedRecipeTask { .. }
        | Action::BeginSelectedRecipeForceTask
        | Action::SelectRecipeTask { .. }
        | Action::PreviewSelectedRecipeTask
        | Action::CancelRecipeTaskPicker
        | Action::SelectSignatureTask { .. }
        | Action::ConfirmSignatureTask
        | Action::CancelSignatureTaskPicker
        | Action::OpenSelectedRecipeProvider
        | Action::BeginSelectedRecipeTaskLog
        | Action::SelectRecipeTaskLog { .. }
        | Action::OpenSelectedRecipeTaskLog
        | Action::CancelRecipeTaskLogPicker
        | Action::BeginSelectedRecipePatchReview
        | Action::SelectRecipePatch { .. }
        | Action::OpenSelectedRecipePatch
        | Action::CancelRecipePatchPicker
        | Action::BeginSelectedRecipeDevtoolModify
        | Action::BeginSelectedRecipeDevtoolStatus
        | Action::DevtoolStatusLoaded(..)
        | Action::BeginSelectedRecipeDevtoolReset => build_cancelled::reduce_actions(app, action),
        Action::BeginSelectedRecipeDevtoolUpdateRecipe
        | Action::BeginSelectedRecipeDevtoolFinish
        | Action::BeginSelectedRecipeDevtoolDeploy
        | Action::BeginSelectedRecipeDependencies
        | Action::BeginDependencyGraph { .. }
        | Action::DependencyGraphLoaded(..)
        | Action::DependencyGraphPartial { .. }
        | Action::DependencyGraphFailed { .. }
        | Action::SelectDependencyGraphNode { .. }
        | Action::SelectDependencyGraphNodeAt { .. }
        | Action::ToggleDependencyGraphReverse
        | Action::CollapseSelectedDependencyGraphNode
        | Action::ExpandSelectedDependencyGraphNode
        | Action::ToggleSelectedDependencyGraphNode
        | Action::BeginDependencyGraphSearch
        | Action::AppendDependencyGraphQuery(..)
        | Action::BackspaceDependencyGraphQuery
        | Action::ClearDependencyGraphQuery
        | Action::FinishDependencyGraphSearch
        | Action::RefreshDependencyGraph
        | Action::OpenSelectedDependencyRecipe
        | Action::OpenSelectedDependencyProvider
        | Action::OpenSelectedDependencyTaskLog
        | Action::BeginSignatureDump(..)
        | Action::RefreshSignatureDump
        | Action::LeaveSignatureWorkspace
        | Action::OpenSignatureProvider
        | Action::SignatureDumpLoaded { .. }
        | Action::SignatureDumpPartial { .. }
        | Action::SignatureDumpFailed { .. }
        | Action::SelectSignatureRecord { .. }
        | Action::SetSelectedSignatureComparisonSide(..)
        | Action::BeginSignatureComparison
        | Action::SignatureComparisonLoaded { .. }
        | Action::SignatureComparisonPartial { .. }
        | Action::SignatureComparisonFailed { .. }
        | Action::BeginPackageInventory
        | Action::RefreshPackageInventory
        | Action::CancelPackageOperation
        | Action::PackageInventoryLoaded { .. }
        | Action::PackageInventoryPartial { .. }
        | Action::PackageInventoryFailed { .. }
        | Action::SelectPackage { .. }
        | Action::BeginPackageSearch
        | Action::AppendPackageQuery(..)
        | Action::BackspacePackageQuery
        | Action::ClearPackageQuery
        | Action::FinishPackageSearch
        | Action::BeginSelectedPackageDetail
        | Action::PackageDetailLoaded { .. }
        | Action::PackageDetailPartial { .. }
        | Action::PackageDetailFailed { .. }
        | Action::OpenPackageDependency { .. }
        | Action::TogglePackageDependencyKind
        | Action::SelectPackageDependency { .. }
        | Action::OpenSelectedPackageDependency
        | Action::BackPackageNavigation => {
            begin_selected_recipe_devtool_update_recipe::reduce_actions(app, action)
        }
        Action::OpenSelectedPackageRecipe
        | Action::OpenSelectedPackageProvider
        | Action::BeginSelectedRecipeMetadata
        | Action::RecipeMetadataLoaded(..)
        | Action::RecipeMetadataFailed { .. }
        | Action::DependenciesLoaded(..)
        | Action::SelectDependency { .. }
        | Action::OpenSelectedDependency
        | Action::ConfirmRecipeTask
        | Action::CancelRecipeTask
        | Action::ConfirmDevtoolModify
        | Action::CancelDevtoolModify
        | Action::ConfirmDevtoolReset
        | Action::CancelDevtoolReset
        | Action::ConfirmDevtoolUpdateRecipe
        | Action::CancelDevtoolUpdateRecipe
        | Action::SelectDevtoolFinishLayer { .. }
        | Action::PreviewDevtoolFinish
        | Action::CancelDevtoolFinish
        | Action::ConfirmDevtoolFinish
        | Action::CancelDevtoolFinishConfirmation
        | Action::AppendDevtoolDeployTarget(..)
        | Action::BackspaceDevtoolDeployTarget
        | Action::PreviewDevtoolDeploy
        | Action::CancelDevtoolDeploy
        | Action::ConfirmDevtoolDeploy
        | Action::CancelDevtoolDeployConfirmation
        | Action::OpenRecipeEditor { .. }
        | Action::SelectRecipeEditorFile { .. }
        | Action::LoadRecipeEditorContent(..)
        | Action::LoadRecipeEditorExternalContent(..)
        | Action::FocusRecipeEditor(..)
        | Action::EditRecipeEditor(..)
        | Action::BeginRecipeEditorSearch
        | Action::AppendRecipeEditorSearch(..)
        | Action::BackspaceRecipeEditorSearch
        | Action::FinishRecipeEditorSearch
        | Action::NextRecipeEditorMatch { .. }
        | Action::ToggleRecipeEditorEditing
        | Action::OpenRecipeEditorExternal
        | Action::AppendRecipeEditor(..)
        | Action::BackspaceRecipeEditor
        | Action::SaveRecipeEditor
        | Action::RecipeEditorSaved
        | Action::BeginRecipeEditorBuild
        | Action::CloseRecipeEditor
        | Action::SelectLayer { .. }
        | Action::OpenSelectedLayer
        | Action::BeginSelectedLayerWorkspaceEditor
        | Action::BeginSelectedLayerBrowser
        | Action::LoadLayerBrowserDirectory { .. }
        | Action::SelectLayerBrowserEntry { .. }
        | Action::LayerBrowserExpand
        | Action::LayerBrowserEnter
        | Action::LayerBrowserUp
        | Action::CloseLayerBrowser
        | Action::RefreshLayerBrowser
        | Action::ToggleLayerBrowserHidden => {
            open_selected_package_recipe::reduce_actions(app, action)
        }
        Action::SetLayerInspectorMode(..)
        | Action::ScrollLayerBrowserPreview { .. }
        | Action::FocusLayerBrowserTree
        | Action::LoadLayerBrowserPreview { .. }
        | Action::EditSelectedLayerBrowserFile
        | Action::BeginLayerRelationships
        | Action::LayerRelationshipsLoaded(..)
        | Action::SelectConfigVariable { .. }
        | Action::BeginSelectedConfigDetail
        | Action::CopySelectedConfigEffective
        | Action::CopySelectedConfigUnexpanded
        | Action::VariableDetailFailed { .. }
        | Action::OpenSelectedConfigSource
        | Action::SelectConfigSource { .. }
        | Action::OpenSelectedConfigSourceChoice
        | Action::CancelConfigSourcePicker
        | Action::OpenConfigScopePicker
        | Action::SelectConfigScope { .. }
        | Action::ConfirmConfigScope
        | Action::CancelConfigScopePicker
        | Action::OpenConfigComparison
        | Action::CloseConfigComparison
        | Action::BeginConfigEdit
        | Action::ToggleConfigEdit
        | Action::AppendConfigEdit(..)
        | Action::BackspaceConfigEdit
        | Action::PreviewConfigEdit
        | Action::CancelConfigEdit
        | Action::ConfirmConfigEdit
        | Action::CancelConfigEditConfirmation
        | Action::ConfigEditWriteSucceeded { .. }
        | Action::ConfigEditWriteFailed { .. }
        | Action::ConfigEditRefreshSucceeded { .. }
        | Action::ConfigEditRefreshFailed { .. }
        | Action::BeginBbmaskEdit
        | Action::ToggleBbmaskEdit
        | Action::AppendBbmask(..)
        | Action::BackspaceBbmask
        | Action::PreviewBbmaskEdit
        | Action::CancelBbmaskEdit
        | Action::ConfirmBbmaskWrite
        | Action::CancelBbmaskWrite
        | Action::BeginMetadataSearch
        | Action::ClearMetadataQuery
        | Action::FinishMetadataSearch
        | Action::Notify(..)
        | Action::ActivateNotification
        | Action::DismissNotification
        | Action::Quit
        | Action::ConfirmQuit
        | Action::CancelQuit
        | Action::WorkspaceLoaded(..)
        | Action::RecipesLoaded(..)
        | Action::LayersLoaded(..)
        | Action::VariableLoaded(..)
        | Action::RecipeSourcesLoaded { .. }
        | Action::HostTelemetryUpdated(..)
        | Action::Failure(..)
        | Action::Tick => set_layer_inspector_mode::reduce_actions(app, action),
    }
}
