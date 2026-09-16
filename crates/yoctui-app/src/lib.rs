//! Application-owned input mapping, keeping terminal concerns outside the reducer.
mod environment_setup;
mod keyboard_prefix;
pub use environment_setup::*;
mod pty_context;
mod pty_devtool;
mod pty_menuconfig;
mod pty_sdk_shell;

pub use keyboard_prefix::{DEFAULT_PREFIX_TIMEOUT, PrefixCommand, PrefixEvent, PrefixState};

pub use pty_context::{
    PtyContextAction, PtyContextAuthority, PtyContextEntry, PtyContextError, PtyContextLaunch,
    VerifiedPtyEnvironment,
};
pub use pty_devtool::{PtyDevtoolAction, PtyDevtoolError, PtyDevtoolPreview, PtyDevtoolRouter};
pub use pty_menuconfig::{
    PtyBitBakeInteractiveTask, PtyInteractiveRecipe, PtyMenuconfigAction, PtyMenuconfigError,
    PtyMenuconfigPreview, PtyMenuconfigRouter,
};
pub use pty_sdk_shell::{PtySdkShellAction, PtySdkShellPreview, PtySdkShellRouter};
use std::time::SystemTime;
use yoctui_bitbake::{
    BackendEvent, DevtoolOutputStream, DevtoolRunnerEvent, QaLayerCapabilityResponse,
    QaLayerRunnerEvent, QaReportAdapterError, QaReportResponse, QaReportScanOutcome,
    QaTaskCapabilityResponse, QemuRunnerEvent, QemuRunnerOutputStream, SdkToolRunnerEvent,
    SecurityMapperRunnerEvent, TestResultImportResponse, TestResultOperation,
    TestResultRunnerEvent, TestRunnerEvent, WicDeviceInventoryResponse, WicRunnerEvent,
    WicRunnerOutputStream,
};
use yoctui_model::{
    Action, App, AppError, BackgroundJobContext, BackgroundJobError, BackgroundJobId,
    BackgroundJobKind, BackgroundJobOutputEntry, BackgroundJobOutputSource, BackgroundJobProgress,
    BackgroundJobResult, BackgroundJobSpec, BuildRequest, DevtoolOperation, FocusTarget,
    FunctionKey, LayerInspectorMode, LayerRelationship, LayerRelationships, MaintenanceAction,
    MaintenanceBuildHistoryField, MaintenanceCleanupField, MaintenanceDialog,
    MaintenanceGitArchiveField, MaintenanceLockedCacheField, MaintenanceReadinessField,
    MaintenanceView, PopupEditorCommand, QaAction, QaDialog, QaReportFailureKind, QaReportRequest,
    QaView, QemuOutputStream, QemuSessionId, RecipeDependencies, Screen, SdkBuildAction, SdkKind,
    SdkOutputStream, SdkSessionId, SecurityAction, SecurityDialog, SecurityOutputStream,
    SecurityView, Severity, SplitAxis, TaskId, TaskInfo, TestComparison, TestWorkspaceView,
    VariableDetail, VariableIdentity, WicCapability, WicOutput, WicOutputStream, WicSessionId,
};
#[cfg(test)]
mod tests;

mod rootfs_mapping;
pub use rootfs_mapping::backend_event_from_rootfs_data;

mod capability_actions;
pub use capability_actions::{
    LayerActionAvailability, RecipetoolActionAvailability, compatibility_layer_actions,
    compatibility_pkgdata_actions, compatibility_recipetool_actions,
};

mod raw_mapping;
pub use raw_mapping::{
    daemon_job_state_from_app, install_daemon_job_replica, raw_execution_event_from_protocol,
    raw_execution_event_to_protocol, raw_execution_request_from_protocol,
    raw_execution_request_to_protocol, raw_execution_result_from_protocol,
    raw_execution_result_to_protocol, raw_execution_snapshot_from_protocol,
    raw_execution_snapshot_to_protocol, raw_form_parameter_selector, raw_form_selected_recipe,
    raw_history_record_from_protocol, raw_output_chunk_from_protocol, raw_output_chunk_to_protocol,
    raw_selector_authority, raw_selector_for_command,
};
use raw_mapping::{
    install_raw_execution_snapshots, install_raw_history_snapshots, raw_form_selector_choice,
};

mod daemon_snapshot;
pub use daemon_snapshot::{
    client_replica_from_daemon, daemon_protocol_snapshot, recover_daemon_model_metadata,
    reduce_daemon_state,
};

mod compatibility_mapping;
pub use compatibility_mapping::{compatibility_model_snapshot, compatibility_workspace_action};
use compatibility_mapping::{
    daemon_bitbake_lifecycle, daemon_compatibility_protocol, daemon_job_kind, daemon_job_lifecycle,
    stable_workspace_hash,
};

mod daemon_client;
pub use daemon_client::DaemonClientSnapshot;

mod daemon_view;
pub use daemon_view::DaemonClientSyncError;
use daemon_view::daemon_client_view;

mod runner_events;
pub use runner_events::{
    WicSessionEvent, qa_layer_capability_action, qa_layer_runner_action, qa_report_error_action,
    qa_report_response_action, qa_task_capability_action, qemu_actions_for_runner_event,
    sdk_actions_for_runner_event, security_actions_for_mapper_event, test_actions_for_runner_event,
    test_result_actions_for_runner_event, test_results_import_action, wic_actions_for_runner_event,
    wic_actions_for_session_event, wic_capability_action, wic_device_inventory_action,
};

mod job_coordinators;
pub use job_coordinators::{BuildJobCoordinator, DevtoolJobCoordinator};

mod backend_mapping;
pub use backend_mapping::model_action_from_backend_event;

mod keyboard;
pub use keyboard::{
    CheckboxInputAction, Input, KeymapInputResult, MenuInputResult, checkbox_input_action,
    collection_scroll_delta, context_menu_activation_input, global_search_action, input_key_stroke,
    keymap_action_for_app, keymap_preferences_action, menu_action, notification_popup_action,
    onboarding_action, terminal_context_action, terminal_workspace_action,
};

mod mouse;
pub use mouse::{
    MouseInput, MouseKind, application_menu_bounds, dashboard_workspace_action,
    firmware_workspace_action, mouse_action, mouse_action_for_app, overview_workspace_action,
    platform_workspace_action, task_workspace_panel_heights, terminal_workspace_dimensions,
    workbench_chrome_heights, workbench_pane_widths, workspace_collection_action,
};

mod raw_input;
pub use raw_input::{
    RawArgvEditorAction, RawModeInputContext, popup_editor_action, raw_argv_editor_action,
    raw_mode_input, reduce_raw_mode_state, validate_raw_argv_editor,
};

mod workspace_input;
pub use workspace_input::{
    build_cancellation_confirmation_action, build_environment_action,
    compatibility_ui_inspector_action, dependency_workspace_action, errors_action, focus_action,
    focus_action_for_app, images_workspace_action, images_workspace_action_for_view,
    internal_logs_action, key_action, log_workspace_action, logs_action, package_workspace_action,
    qa_dialog_action, qa_workspace_action, quit_confirmation_action, sdk_build_confirmation_action,
    sdk_cancellation_confirmation_action, sdk_native_confirmation_action, sdk_native_dialog_action,
    sdk_publish_confirmation_action, sdk_publish_dialog_action, sdk_workspace_action,
    security_dialog_action, security_workspace_action, settings_action,
    signature_task_picker_action, signature_workspace_action, tasks_action,
    testing_workspace_action,
};

mod maintenance_input;
pub use maintenance_input::{maintenance_dialog_action, maintenance_workspace_action};

mod dialog_input;
pub use dialog_input::{
    config_compare_dialog_action, config_edit_confirmation_action, config_edit_dialog_action,
    config_scope_picker_action, config_source_picker_action, config_workspace_action,
    devtool_deploy_confirmation_action, devtool_deploy_dialog_action,
    devtool_finish_confirmation_action, devtool_finish_picker_action,
    devtool_modify_confirmation_action, devtool_reset_confirmation_action,
    devtool_update_confirmation_action, dtc_compile_dialog_action, image_console_dialog_action,
    layer_tree_action, qemu_cancellation_confirmation_action, qemu_launch_confirmation_action,
    qemu_launch_dialog_action, recipe_editor_action, recipes_workspace_action,
    terminal_launch_dialog_action, test_cancellation_confirmation_action,
    test_comparison_confirmation_action, test_comparison_dialog_action,
    test_comparison_workspace_action, test_junit_confirmation_action, test_junit_dialog_action,
    test_launch_confirmation_action, test_launch_dialog_action, test_result_import_dialog_action,
    test_results_workspace_action, wic_cancellation_confirmation_action,
    wic_create_confirmation_action, wic_create_dialog_action, wic_device_picker_action,
    wic_write_confirmation_action, wic_write_phrase_action,
};
