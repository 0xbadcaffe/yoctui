//! Domain model and pure state transitions. BitBake remains authoritative.
mod action_catalog;
mod background_activity;
mod build_cache;
pub use background_activity::BackgroundActivity;
pub use build_cache::{BuildCacheState, SstateSummary};
mod bitbake_layers;
mod bitbake_restart;
mod checkbox;
mod compatibility;
mod compatibility_catalog;
mod compatibility_ui;
mod daemon_state;
mod dashboard;
mod devtool_utility;
mod embedded_shell;
mod environment_setup;
mod focus;
mod global_search;
mod image;
mod image_console;
mod internal_log;
mod keymap;
mod list_tree;
mod maintenance;
mod menu;
mod onboarding;
mod overview;
mod package;
mod pane_layout;
mod platform;
mod preferences;
mod progress;
mod project_profile;
mod pty_multi;
mod pty_session;
mod qa;
mod qemu;
mod raw_catalog_builtin;
mod raw_mode;
mod recipetool;
mod rootfs;
mod scroll;
mod sdk;
mod security;
mod telemetry_projection;
mod terminal_emulation;
mod terminal_workbench;
mod testing;
mod textarea;
mod utility_compatibility;
mod utility_menu;
mod wic;
mod widget_projection;
mod workspace_compatibility;
mod yocto_utility;

pub use action_catalog::*;
pub use bitbake_layers::*;
pub use bitbake_restart::*;
pub use checkbox::*;
pub use compatibility::*;
pub use compatibility_catalog::*;
pub use compatibility_ui::*;
pub use daemon_state::*;
pub use dashboard::*;
pub use devtool_utility::*;
pub use embedded_shell::*;
pub use environment_setup::*;
pub use focus::*;
pub use global_search::*;
pub use image::*;
pub use image_console::*;
pub use internal_log::*;
pub use keymap::*;
pub use list_tree::*;
pub use maintenance::*;
pub use menu::*;
pub use onboarding::*;
pub use overview::*;
pub use package::*;
pub use pane_layout::*;
pub use platform::*;
pub use preferences::*;
pub use progress::*;
pub use project_profile::*;
pub use pty_multi::*;
pub use pty_session::*;
pub use qa::*;
pub use qemu::*;
pub use raw_catalog_builtin::{
    RAW_BUILTIN_CATEGORY_COUNT, RAW_BUILTIN_COMMAND_COUNT, RAW_BUILTIN_EXECUTABLE_COUNT,
    RAW_REFERENCE_SHA256,
};
pub use raw_mode::*;
pub use recipetool::*;
pub use rootfs::*;
pub use scroll::*;
pub use sdk::*;
pub use security::*;
use serde::{Deserialize, Serialize};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
    fmt,
    path::{Component, Path, PathBuf},
    time::{Duration, SystemTime},
};
pub use telemetry_projection::*;
pub use terminal_emulation::*;
pub use terminal_workbench::*;
pub use testing::*;
pub use textarea::*;
use thiserror::Error;
pub use utility_compatibility::*;
pub use utility_menu::*;
pub use wic::*;
pub use widget_projection::*;
pub use workspace_compatibility::*;
pub use yocto_utility::*;

use yoctui_utils::push_bounded;

use yoctui_utils::append_truncation_marker;
pub use yoctui_utils::format_duration;
#[cfg(test)]
mod tests;

mod navigation_types;
pub use navigation_types::{
    AnimationSpeed, AppError, ClientAccessOrigin, CommandId, CommandPaletteMode,
    FUNCTION_SHORTCUTS, FocusTarget, FunctionKey, FunctionShortcut, FunctionShortcutRoute,
    InspectorMode, MAX_COMMAND_PALETTE_QUERY_CHARS, NAVIGATOR_GROUPS, NavigatorGroupRange,
    PaletteCommand, Screen, Theme, function_shortcut_action, notification_requires_acknowledgement,
};
use navigation_types::{
    NAVIGATOR_COMPATIBILITY_DESTINATIONS, NAVIGATOR_SCREENS, navigator_group_for_selection,
};

mod build_environment_types;
pub use build_environment_types::{
    BuildEnvironmentClonePlan, BuildEnvironmentCloneRequest, BuildEnvironmentDraft,
    BuildEnvironmentField, BuildEnvironmentProfile, BuildEnvironmentState, BuildRequest,
    BuildStatus, Severity,
};

mod task_types;
pub use task_types::{
    CompletedTask, TaskFilterField, TaskFilters, TaskId, TaskInfo, TaskInspectorRef,
    TaskProjectionCache, TaskProjectionCacheCell, TaskProjectionKey, TaskRow, TaskRowRef,
    TaskState, TaskStateFilter, TaskStats,
};

mod devtool_types;
pub use devtool_types::{
    DevtoolDeployDraft, DevtoolDeployPlan, DevtoolDeployRequest, DevtoolFinishPicker,
    DevtoolFinishPlan, DevtoolFinishRequest, DevtoolOperation, DevtoolOperationError,
    DevtoolPatchPicker, DevtoolPatchPlan, DevtoolResetPlan, DevtoolUndeployDraft,
    DevtoolUndeployPlan, DevtoolUpgradePlan,
};

mod workspace_types;
use workspace_types::MAX_COMPLETED_TASKS;
pub use workspace_types::{
    ConfigCopyValue, DiagnosticInfo, LogEntry, MAX_ACTIVE_TASKS, VariableDetail, VariableIdentity,
    VariableOperation, Workspace,
};

mod telemetry_types;
pub use telemetry_types::{
    BitBakeCoexistenceDiagnostic, BitBakeCoexistencePressure, HOST_TELEMETRY_HISTORY_SAMPLES,
    HOST_TELEMETRY_SAMPLE_PERIOD_SECONDS, HostTelemetry, TELEMETRY_PROVENANCE, TelemetryHistory,
    TelemetryHostSupport, TelemetryMetric, TelemetryProvenance, TelemetrySource, TelemetryUnit,
    bitbake_coexistence_diagnostic, parse_parallel_make_jobs,
};

mod recipe_types;
pub use recipe_types::{
    ConfigComparison, ConfigComparisonField, ConfigComparisonOutcome, ConfigEditRequest,
    ConfigScopePicker, ConfigSourceChoice, ConfigSourcePicker, DevtoolAction, DevtoolCapability,
    DevtoolGitState, DevtoolStatus, DevtoolStatusError, DevtoolWorkspace, Layer, Recipe,
    RecipeBuildStatus, RecipeEditor, RecipeEditorFocus, RecipeIdentity, RecipeMetadata,
    RecipePatchPicker, RecipeTaskLogChoice, RecipeTaskLogPicker, RecipeTaskPicker,
    RecipeWorkspaceStatus, SignatureTaskPicker, SourceLanguage,
};

mod editor_types;
pub use editor_types::MAX_RECIPE_EDITOR_FILES;
use editor_types::apply_recipe_editor_command;
pub use editor_types::{
    Dialog, PopupEditor, PopupEditorCommand, TransientStatus, TransientStatusKind,
};

mod background_jobs;
pub use background_jobs::{
    BackgroundJob, BackgroundJobContext, BackgroundJobError, BackgroundJobId, BackgroundJobKind,
    BackgroundJobOutputEntry, BackgroundJobOutputSource, BackgroundJobProgress,
    BackgroundJobResult, BackgroundJobSpec, BackgroundJobStatus, BackgroundJobs, BuildRecord,
    BuildState, BuildSummary, JobHistoryRowRef, JobSummary, MAX_SIGNATURE_DIFFERENCES,
    MAX_SIGNATURE_RECORDS,
};

mod dependency_graph;
pub use dependency_graph::{
    DEPENDENCY_GRAPH_MAX_QUERY_BYTES, DependencyEdge, DependencyEdgeKind, DependencyGraph,
    DependencyGraphProjection, DependencyGraphState, DependencyNode, DependencyNodeId,
    DependencyNormalizationReport, DependencyPathResult, DependencyProjectionRow,
    RecipeDependencies,
};

mod signatures;
pub use signatures::{
    SignatureComparisonRequest, SignatureComparisonSide, SignatureComparisonState,
    SignatureDifference, SignatureDifferenceCategory, SignatureDifferenceReport,
    SignatureDumpState, SignatureIdentity, SignatureNormalizationReport, SignatureRecord,
    SignatureTarget, SignatureValue, compare_signature_records, normalize_signature_differences,
    normalize_signature_records,
};

mod layer_browser;
pub use layer_browser::{
    GitFileState, ImagePicker, LayerBrowser, LayerBrowserEntry, LayerInspectorMode,
    LayerRelationship, LayerRelationships, MAX_BUILD_HISTORY, PreviewKind, RecipePicker,
    RecipePickerPurpose,
};

mod log_state;
pub use log_state::{LogState, LogTimeRange, LogWindow};

mod app_state;
pub use app_state::{App, centered_viewport_range};

mod actions;
pub use actions::{Action, ObservedTaskTiming, TaskEvent};

mod task_updates;
pub use task_updates::format_log_details;
use task_updates::{
    apply_task_batch, apply_task_event, archive_unfinished_tasks, clamp_task_selection,
    insert_log_batch, insert_log_entry, insert_system_log, mark_build_running_from_task_activity,
    prepare_build,
};

mod log_export;
pub use log_export::{
    LogExport, MAX_LOG_COPY_BYTES, MAX_LOG_EXPORT_BYTES, THEMES, command_action,
    format_log_details_bounded, format_log_export,
};
use log_export::{
    close_dialog, cycle_theme, enqueue_build_completion, is_pane_focus, modal_focus, open_dialog,
    replace_dialog, selected_correlated_log_id, synchronize_focus,
};

mod recipe_operations;
use recipe_operations::{
    begin_recipe_task, begin_recipe_task_for, begin_terminal_creation, devtool_gitui_request,
    devtool_terminal_request, open_terminal_launch, recipe_matches_query,
    select_first_matching_layer_entry, select_first_matching_recipe, selected_recipe_identity,
};

mod config_operations;
pub use config_operations::{
    EDITABLE_CONFIG_VARIABLES, config_comparison, config_edit_assignment,
    config_edit_disabled_reason, config_source_disabled_reason, selected_config_copy_value,
    validate_config_edit_request,
};
use config_operations::{
    config_edit_context, filtered_config_identities, popup_toml_document, popup_toml_fields,
    popup_toml_value, resolve_config_source, selected_config_identity, selected_config_sources,
};

mod inventory_updates;
use inventory_updates::{
    begin_image_artifact_inventory, begin_rootfs_composition, begin_signature_dump,
    image_artifact_operation_is_loading, rootfs_group_rows, set_dependency_graph,
    set_rootfs_composition, set_signature_dump, signature_comparison_inputs,
    signature_operation_is_loading,
};

mod session_updates;
use session_updates::{
    MAX_QEMU_SESSIONS, begin_sdk_artifact_inventory, begin_test_result_import,
    current_wic_write_preview, is_uncompressed_wic_path, mutate_qemu_session, mutate_sdk_session,
    mutate_test_session, mutate_wic_session, next_qemu_session_id, note_stale_qemu_event,
    note_stale_sdk_event, note_stale_test_event, note_stale_wic_event, qemu_background_job_id,
    qemu_job_id, queue_sdk_session, queue_test_session, queue_wic_session,
    reconcile_wic_device_selection, reconcile_wic_output_selection, sdk_job_id,
    set_sdk_artifact_selection_to_current_or_first, set_test_comparison_selection,
    set_test_result_selection_to_current_or_first, test_comparison_inputs_exist,
    test_comparison_request_is_current, test_job_id, test_junit_request_is_current,
    test_launch_draft, test_preview_is_current, test_result_request_is_current, wic_job_id,
};

mod image_updates;
use image_updates::{
    append_package_normalization_limitations, begin_package_detail, begin_package_inventory,
    current_collection_edge_action, normalize_package_limitations, package_detail_is_empty,
    package_operation_is_loading, select_package_identity, set_image_artifact_inventory,
    set_image_artifact_selection_to_current_or_first, set_package_inventory,
    set_package_selection_to_current_or_first,
};

mod reducer;
pub use reducer::update;

mod effects;
pub use effects::Effect;
use effects::next_filter;

mod source_git;
pub use source_git::*;

mod offline;

mod saved_builds;
pub use saved_builds::*;

mod error_workspace;
pub use error_workspace::*;
