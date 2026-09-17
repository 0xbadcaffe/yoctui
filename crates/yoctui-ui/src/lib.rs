//! Rendering only; no backend parsing or mutation lives in widgets.
use yoctui_utils::format_duration;
mod dialogs;
mod environment_setup;
mod layout;
mod overview;
pub mod primitives;
mod shell;
mod telemetry;
mod theme;
mod widgets;
mod workspaces;

use dialogs::*;
use layout::*;
use overview::*;
use shell::*;
use telemetry::*;
use theme::*;
use widgets::*;
use workspaces::*;

use primitives::{
    ActionListItem, ActionListStyles, BoundedScrollIndicator, DialogShell, DialogStyles,
    DialogTone, PaneShell, PaneStyles, ResponsiveColumn, StateKind, StateView, StatusTone,
    WidgetRenderOptions, WidgetStyles, action_list, action_list_plain, bounded_dialog_rect,
    render_history_chart, responsive_columns, status_label,
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Sparkline, Table, Wrap},
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
mod yocto_logs;

use tui_piechart::{LegendPosition, PieChart, PieSlice, Resolution};
use tui_term::widget::PseudoTerminal;
use tui_tree_widget::{Tree, TreeItem, TreeState};
use yoctui_model::{
    App, BackgroundJobKind, BackgroundJobOutputSource, BackgroundJobStatus, BuildEnvironmentState,
    BuildStatus, CommandCenterProjection, CommandPaletteMode, CompatibilityUiAuthorityStatus,
    CompatibilityUiCapabilityRow, CompatibilityUiCapabilityState, CompatibilityUiFilter,
    ConfigCopyValue, DashboardProjection, DependencyEdgeKind, DependencyGraph,
    DependencyGraphState, DependencyNodeId, DependencyPathResult, DevtoolAction, DevtoolCapability,
    DevtoolGitState, DevtoolStatus, DevtoolStatusError, DevtoolWorkspace, Dialog,
    FUNCTION_SHORTCUTS, FocusTarget, FunctionKey, FunctionShortcutRoute, GitFileState,
    ImageArtifactField, ImageArtifactInventoryState, ImageConsoleDialog, ImageConsoleField,
    ImageConsoleMode, ImagesView, InternalLogLevel, JobHistoryRowRef, LayerBrowser,
    LayerBrowserEntry, LayerInspectorMode, LogWorkspaceView, MaintenanceCapability,
    MaintenanceCapabilitySnapshot, MaintenanceDialog, MaintenanceIntegrationDiagnostics,
    MaintenanceIntegrationsSnapshot, MaintenanceOperation, MaintenanceOperationPreview,
    MaintenanceServiceDiagnostics, MaintenanceSessionStatus, MaintenanceTool,
    MaintenanceToolCapability, MaintenanceToolInterface, MaintenanceView, NAVIGATOR_GROUPS,
    PackageDetailState, PackageField, PackageIdentity, PackageInventoryState, PaneNode,
    PlatformInventoryState, PlatformWorkbench, PreviewKind, QaCapability, QaCheckAvailability,
    QaCheckFamily, QaDialog, QaFindingStatus, QaLayerCapability, QaLayerRunCapability,
    QaOutputStream, QaReportFailureKind, QaReportInventoryState, QaSessionStatus, QaStatusFilter,
    QaView, QemuCapability, QemuDisplayMode, QemuLaunchDialog, QemuLaunchField, QemuLaunchPreview,
    QemuNetworkingMode, QemuSerialMode, QemuSessionId, Recipe, RecipeBuildStatus, RecipeEditor,
    RecipeIdentity, RootfsCompositionState, RootfsEntryKind, RootfsGroupIdentity, Screen,
    SdkArtifactInventoryState, SdkArtifactKind, SdkBuildAction, SdkKind, SdkNativeDialog,
    SdkNativeField, SdkNativeMode, SdkNativePreview, SdkOperation, SdkPublishDraft,
    SdkPublishPreview, SdkSessionId, SdkToolCapability, SecurityCapability, SecurityDialog,
    SecurityInventoryState, SecurityOperation, SecurityOutputStream, SecurityReport, SecurityScope,
    SecuritySessionStatus, SecurityView, Severity, SignatureComparisonState,
    SignatureDifferenceCategory, SignatureDumpState, SpdxArtifactKind, SplitAxis, SymbolPreference,
    TaskInspectorRef, TaskRowRef, TaskState, TelemetryMetric, TelemetrySeriesProjection,
    TestComparisonCategory, TestComparisonState, TestExecutableCapability, TestJunitExportState,
    TestLaunchDialog, TestLaunchField, TestLaunchPreview, TestResultInventoryState,
    TestWorkspaceView, Theme, TransientStatusKind, VariableIdentity, WicCapability, WicCompression,
    WicCreateDialog, WicCreateField, WicCreatePreview, WicDevice, WicDeviceInventoryState,
    WicDevicePickerDialog, WicKickstart, WicOperation, WicOutputInventoryState, WicSessionId,
    WicWritePhraseDialog, WicWritePreview, WorkspaceAvailabilityState, WorkspaceDestination,
    compatibility_ui_workspace_destination_action_availability, config_comparison,
    config_edit_disabled_reason, config_source_disabled_reason,
    notification_requires_acknowledgement, selected_config_copy_value,
};
#[cfg(test)]
mod task_layout_tests;
#[cfg(test)]
mod tests;

mod terminal_render;
pub use terminal_render::startup_activity_symbol;
use terminal_render::{
    LITERAL_REFERENCE_WIDTH, SearchExit, SearchNavigation, WIDE_WORKBENCH_MIN_WIDTH,
    render_terminal_replica_content, search_line,
};

mod semantic_theme;
pub use semantic_theme::SemanticTheme;
use semantic_theme::ThemePalette;

mod source_render;
use source_render::{numbered_source_preview, source_preview};

mod footer;
use footer::{
    compatibility_destination_detail, compatibility_workspace_actions, footer_rail_shortcuts,
};

mod header;
use header::{
    clock_text, daemon_lifecycle_label, daemon_lifecycle_tone, workbench_footer, workbench_header,
};

mod render;
pub use render::{render, render_at};

mod popup_render;
use popup_render::{
    build_environment_clone_editor, build_environment_clone_review, build_environment_editor,
    dialog_compatibility_overlay, notification_popup, popup_notification, theme_picker,
};

mod palette_render;
use palette_render::command_palette;

mod terminal_workspace;
use terminal_workspace::{
    pane_block, pane_styles, pane_switcher, terminal_session_inspector_text,
    terminal_sessions_workspace,
};

mod signature_render;
use signature_render::{signature_detail_text, signature_records, signatures_workspace};

mod navigator_render;
use navigator_render::navigator;

mod raw_render;
use raw_render::raw_mode_workspace;

mod raw_catalog_render;
use raw_catalog_render::{
    raw_availability_label, raw_category_browser, raw_command_help_text, raw_command_list,
    raw_command_template, raw_parameter_kind_label, raw_parameter_presence_label,
};

mod inspector_render;
use inspector_render::{
    InspectorDocumentSections, bounded_status_line, inspector_action_styles, inspector_document,
    inspector_related_paths, status_tone_style, system_status_document, system_status_text,
    task_inspector_actions, task_inspector_context, task_inspector_primary,
    task_inspector_recent_lines,
};

mod inspector_workspace;
use inspector_workspace::inspector;

mod editor_render;
use editor_render::{build_completion_popup, recipe_editor};

mod telemetry_gauges;
use telemetry_gauges::{
    format_bytes_pair, format_bytes_pair_with, render_disk_gauge, render_disk_io_projection,
    render_network_io_projection, render_ram_gauge,
};

mod telemetry_strip;
use telemetry_strip::{
    render_compact_telemetry_strip, render_tasks_context_zoom, render_telemetry_strip,
};

mod dashboard_dials;
mod dashboard_render;
use dashboard_render::{command_center_context_line, dashboard_recent_work_line};

mod task_render;
use task_render::{render_build_summary, render_task_log, render_task_table, task_state_label};

mod job_history;
use job_history::{
    background_job_style, daemon_job_kind_label, daemon_job_status_label, job_context, job_elapsed,
    job_history_cell, job_history_columns, job_history_row_active, job_kind_label,
    job_status_label, job_summary_label, render_job_history, tasks_workspace,
};

mod qa_render;
use qa_render::{
    qa_dialog, qa_inspector_text, qa_workspace, sdk_inventory_root, sdk_kind_label, sdk_type_label,
};

mod security_render;
use security_render::{
    security_dialog, security_error_style, security_info_style, security_inspector_text,
    security_warning_style, security_workspace,
};

mod testing_render;
pub use testing_render::checkbox_text;
use testing_render::{
    popup_editor_text, test_cancellation_confirmation, test_comparison_confirmation,
    test_comparison_dialog, test_junit_confirmation, test_junit_dialog, test_launch_confirmation,
    test_launch_dialog, test_result_import_dialog, testing_inspector_text, testing_workspace,
    textarea_mode_label, toml_popup_editor,
};

mod sdk_render;
use sdk_render::{
    image_console_dialog, platform_inspector_text, platform_workspace,
    qemu_cancellation_confirmation, qemu_launch_confirmation, qemu_launch_dialog,
    sdk_build_confirmation, sdk_cancellation_confirmation, sdk_inspector_text,
    sdk_native_confirmation, sdk_native_dialog, sdk_publish_confirmation, sdk_publish_dialog,
    sdk_workspace, wic_cancellation_confirmation, wic_create_confirmation, wic_create_dialog,
    wic_device_picker, wic_limitations, wic_partition_summary, wic_write_confirmation,
    wic_write_phrase_dialog,
};

mod image_render;
use image_render::{images_tabs_line, images_workspace, rootfs_state_lines};

mod rootfs_render;
use rootfs_render::{
    rootfs_filesystem_workspace, rootfs_group_label, rootfs_packages_workspace,
    rootfs_workspace_shell,
};

mod rootfs_services;
use rootfs_services::{rootfs_dbus_workspace, rootfs_systemd_workspace, rootfs_udev_workspace};

mod image_inspector;
use image_inspector::image_artifact_inspector_text;

mod emulation_inspector;
use emulation_inspector::{qemu_capability_text, qemu_session_text, wic_inspector_text};

mod compatibility_render;
use compatibility_render::{
    compatibility_inspector_text, compatibility_workspace, compatibility_workspace_state_label,
    compatibility_workspace_state_style,
};

mod settings_render;
use settings_render::{keymap_preferences_overlay, onboarding_overlay, settings_workspace};

mod menu_render;
use menu_render::menu_overlay;

mod history_render;
use history_render::{build_environment_workspace, build_history, job_history_detail};

mod dependency_render;
use dependency_render::{dependencies, dependency_inspector, layer_relationships};

mod log_render;
use log_render::{
    case_insensitive_ranges, compact_log_activity, format_bytes, list_or_none, log_severity_label,
    log_workspace_tabs, logs,
};

mod internal_log_render;
use internal_log_render::internal_logs;

mod error_render;
use error_render::{diagnostic_detail, errors};

mod recipe_render;
use recipe_render::{recipe_build_state, recipe_inspector, recipe_workspace_state};

mod package_render;
use package_render::{
    layer_browser, layer_entry_metadata, layers, package_inspector_text, packages_workspace,
    recipes,
};

mod config_render;
use config_render::{bbmask, config, config_inspector};

mod maintenance_render;
use maintenance_render::{maintenance_inspector_text, maintenance_workspace};

mod raw_dialog;
pub use raw_dialog::render_raw_execution_preview;
use raw_dialog::{indexed_arguments, raw_command_form_dialog};

mod maintenance_dialog_render;
use maintenance_dialog_render::{
    bbmask_assignment, help, maintenance_dialog, maintenance_operation_label,
};

mod saved_builds;
