use maintenance_cli::MaintenanceCliCoordinator;
#[cfg(unix)]
use yoctui_utils::unix_ms;
mod build_archive;

use anyhow::{Context, Result};

use clap::{Parser, Subcommand, ValueEnum};

use crossterm::{
    cursor::{Hide, Show},
    event::{
        self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
        Event, KeyCode, KeyEvent, KeyModifiers,
    },
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, SetTitle, disable_raw_mode, enable_raw_mode,
    },
};

use ratatui::Terminal;

use render_scheduler::{
    ELAPSED_REFRESH_INTERVAL, RenderCause, RenderScheduler, animation_interval,
    has_live_elapsed_time, has_visible_indeterminate_activity, ordinary_frame_interval,
};

use serde::{Deserialize, Serialize};

use std::{
    cell::Cell,
    collections::{BTreeMap, HashMap},
    env, fs,
    fs::OpenOptions,
    io,
    io::{IsTerminal as _, Read as _, Write as _},
    path::{Path, PathBuf},
    process::{Command as ProcessCommand, Stdio},
    time::{Duration, Instant, SystemTime},
};

#[cfg(unix)]
use std::{
    ffi::CString,
    os::unix::{ffi::OsStrExt, fs::PermissionsExt, process::CommandExt},
};

use telemetry_scheduler::{
    client_telemetry_interval, client_telemetry_visible, daemon_telemetry_interval,
};

#[cfg(unix)]
use tokio::signal::unix::{SignalKind, signal};

use tracing_subscriber::prelude::*;

use yoctui_app::{
    BuildJobCoordinator, DevtoolJobCoordinator, Input, MenuInputResult, MouseInput, MouseKind,
    PrefixCommand, PrefixEvent, PrefixState, build_cancellation_confirmation_action,
    build_environment_action, collection_scroll_delta, compatibility_ui_inspector_action,
    compatibility_workspace_action, config_compare_dialog_action, config_edit_confirmation_action,
    config_scope_picker_action, config_source_picker_action, config_workspace_action,
    daemon_job_state_from_app, daemon_protocol_snapshot, dashboard_workspace_action,
    dependency_workspace_action, devtool_deploy_confirmation_action, devtool_deploy_dialog_action,
    devtool_finish_confirmation_action, devtool_finish_picker_action,
    devtool_modify_confirmation_action, devtool_reset_confirmation_action,
    devtool_update_confirmation_action, dtc_compile_dialog_action, errors_action,
    firmware_workspace_action, focus_action_for_app, global_search_action,
    image_console_dialog_action, images_workspace_action_for_view, keymap_action_for_app,
    keymap_preferences_action, log_workspace_action, maintenance_dialog_action,
    maintenance_workspace_action, menu_action, model_action_from_backend_event,
    mouse_action_for_app, notification_popup_action, onboarding_action, overview_workspace_action,
    package_workspace_action, platform_workspace_action, popup_editor_action, qa_dialog_action,
    qa_layer_capability_action, qa_layer_runner_action, qa_report_error_action,
    qa_report_response_action, qa_task_capability_action, qa_workspace_action,
    qemu_actions_for_runner_event, qemu_cancellation_confirmation_action,
    qemu_launch_confirmation_action, qemu_launch_dialog_action, quit_confirmation_action,
    raw_mode_input, recipe_editor_action, recover_daemon_model_metadata,
    sdk_actions_for_runner_event, sdk_build_confirmation_action,
    sdk_cancellation_confirmation_action, sdk_native_confirmation_action, sdk_native_dialog_action,
    sdk_publish_confirmation_action, sdk_publish_dialog_action, sdk_workspace_action,
    security_actions_for_mapper_event, security_dialog_action, security_workspace_action,
    settings_action, signature_task_picker_action, signature_workspace_action, tasks_action,
    terminal_launch_dialog_action, test_actions_for_runner_event,
    test_cancellation_confirmation_action, test_comparison_confirmation_action,
    test_comparison_dialog_action, test_comparison_workspace_action,
    test_junit_confirmation_action, test_junit_dialog_action, test_launch_confirmation_action,
    test_launch_dialog_action, test_result_actions_for_runner_event,
    test_result_import_dialog_action, test_results_import_action, test_results_workspace_action,
    testing_workspace_action, wic_actions_for_runner_event, wic_cancellation_confirmation_action,
    wic_create_confirmation_action, wic_create_dialog_action, wic_device_picker_action,
    wic_write_confirmation_action, wic_write_phrase_action, workspace_collection_action,
};

use yoctui_bitbake::{
    BackendEvent, BitBakeBackend, BridgeBackend, BridgeProcessPriority, BuildEnvironmentAdapter,
    DevtoolCommandSpec, DevtoolInspector, DevtoolJobRunner, DevtoolRunnerEvent,
    ImageArtifactAdapter, ImageArtifactCancellation, PackageDataAdapter, PackageDataCancellation,
    PlatformArtifactAdapter, ProcessBackend, QaConfiguredLayerInput, QaFamilyTaskBinding,
    QaLayerCapabilityInput, QaLayerCapabilityInspector, QaLayerCommandSpec, QaLayerJobRunner,
    QaLayerRunnerEvent, QaReportAdapter, QaReportAdapterError, QaReportCancellation,
    QaReportCandidate, QaReportOrigin, QaReportRootInput, QaReportScanInput, QaTaskCapabilityInput,
    QaTaskCapabilityInspector, QaTaskScopeInput, QemuAdapterError, QemuCapabilityInspector,
    QemuCommandSpec, QemuJobRunner, QemuRunnerEvent, RootfsCompositionAdapter,
    RootfsCompositionCancellation, RootfsCompositionSources, SdkArtifactAdapter,
    SdkArtifactCancellation, SdkArtifactScanOutcome, SdkToolAdapter, SdkToolAdapterError,
    SdkToolCommandSpec, SdkToolJobRunner, SdkToolRunnerEvent, SecurityCapabilityInput,
    SecurityCapabilityInspector, SecurityMapperCommandSpec, SecurityMapperJobRunner,
    SecurityMapperRunnerEvent, SecurityReportAdapter, SecurityReportAdapterError,
    SecurityReportCancellation, SecurityReportScanOutcome, SignatureAdapter, SignatureCancellation,
    TestResultAdapter, TestResultJob, TestResultOperation, TestResultRunnerEvent,
    TestRunnerAdapter, TestRunnerEvent, TestRunnerJob, VariableValue, WicAdapterError,
    WicCapabilityInspector, WicCreateCommandSpec, WicDeviceInspector, WicDeviceInventoryResponse,
    WicJobRunner, WicRunnerEvent,
};

use yoctui_model::{
    Action, AnimationSpeed, App, AppError, BitBakeCoexistenceDiagnostic,
    BitBakeCoexistencePressure, BuildRequest, BuildStatus, ClientAccessOrigin, ConfigEditRequest,
    DevtoolOperation, DevtoolWorkspace, Dialog, Effect, GitFileState, HostTelemetry,
    ImageArtifactInventoryState, ImageArtifactRequest, LayerBrowserEntry, LayerInspectorMode,
    LayerRelationship, LayerRelationships, OnboardingProgress, PackageDetailRequest,
    PackageInventoryRequest, PlatformComponent, PlatformInventory, PreviewKind, QaAction,
    QaCheckFamily, QaCheckId, QaEffect, QaFindingScope, QaLayerIdentity, QaLayerSessionId,
    QaReportFormat, QaReportIdentity, QaReportRequest, QaScope, QaSessionId, QaSessionStatus,
    QaSourceLocation, QemuCapability, QemuLaunchDraft, QemuLaunchPreview, QemuLaunchRequest,
    QemuSessionId, RecipeIdentity, RootfsCompositionRequest, Screen, SdkArtifactInventoryRequest,
    SdkNativePreview, SdkOperation, SdkPublishPreview, SdkSessionId, SdkToolCapability,
    SecurityAction, SecurityEffect, SecurityOperation, SecurityReportRequest, SecurityScope,
    SecuritySessionId, SecuritySessionStatus, Severity, SignatureComparisonRequest,
    SignatureTarget, TEXTAREA_MAX_BYTES, TestComparison, TestOperation, TestSessionId,
    TestWorkspaceView, TextAreaRevision, Theme, VariableDetail, VariableIdentity, WicCapability,
    WicCreateDraft, WicCreatePreview, WicCreateRequest, WicDeviceInventoryRequest, WicOperation,
    WicSessionId, WorkbenchPreferences, bitbake_coexistence_diagnostic, update,
    validate_config_edit_request, validate_raw_favorites,
};

use yoctui_ui::render;

#[cfg(unix)]
#[cfg_attr(not(test), allow(dead_code))]
mod client_runtime;

#[cfg(unix)]
#[cfg_attr(not(test), allow(dead_code))]
mod client_transport;

mod clone_operation;

#[cfg(unix)]
mod daemon_bitbake;

#[cfg(unix)]
mod daemon_build;

#[cfg(unix)]
#[cfg_attr(not(test), allow(dead_code))]
mod daemon_compatibility;

#[cfg(unix)]
mod daemon_devtool;

#[cfg(unix)]
mod daemon_job_ids;

#[cfg(unix)]
mod daemon_maintenance;

#[cfg(unix)]
mod daemon_metadata;

#[cfg(unix)]
mod daemon_pty;

#[cfg(unix)]
mod daemon_qa;

#[cfg(unix)]
mod daemon_qemu;

#[cfg(unix)]
mod daemon_raw;

#[cfg(unix)]
mod daemon_rootfs;

#[cfg(unix)]
mod daemon_sdk;

#[cfg(unix)]
mod daemon_security;

#[cfg(unix)]
mod daemon_test;

#[cfg(unix)]
mod daemon_wic;

mod environment_operation;

mod environment_setup;

mod global_search;

mod internal_tracing;

mod maintenance_cli;

#[cfg(unix)]
#[cfg_attr(not(test), allow(dead_code))]
mod pty_attach;

#[cfg(test)]
mod pty_workflow_tests;

mod render_scheduler;

mod source_git;

mod telemetry_scheduler;

use global_search::{
    GlobalSearchCancellation, GlobalSearchPlan, GlobalSearchScanResult, scan_global_content,
};

// Two workers keep the reactor responsive while one worker is inside one of the
// bounded synchronous terminal/listener polls. More workers add idle scheduler
// threads without improving those bounded waits; expensive filesystem and
// process work is dispatched through `spawn_blocking` at its call sites.
fn uses_interactive_terminal(cli: &Cli) -> bool {
    !cli.headless
        && matches!(
            cli.command,
            None | Some(Command::Attach | Command::Build { .. })
        )
}

#[tokio::main(worker_threads = 2)]
async fn main() -> Result<()> {
    install_panic_hook();
    let cli = Cli::parse();
    if let Some(Command::Daemon { command }) = &cli.command {
        return daemon_cli(command.clone()).await;
    }
    if matches!(&cli.command, Some(Command::Sessions)) {
        return daemon_sessions();
    }
    if let Some(Command::Session { command }) = &cli.command {
        return daemon_session_command(command.clone());
    }
    let session = read_session(session_path(config_path(&cli).as_deref()).as_deref())?;
    let config = resolve_config(&cli, &session)?;
    let (internal_tracing_layer, internal_tracing_capture) =
        internal_tracing::bounded_channel(1_024);
    let tracing_filter = tracing_subscriber::EnvFilter::try_new(config.log_level.clone())
        .context("invalid Yoctui tracing filter")?;
    tracing_subscriber::registry()
        .with(tracing_filter)
        .with(
            (!uses_interactive_terminal(&cli))
                .then(|| tracing_subscriber::fmt::layer().with_writer(std::io::stderr)),
        )
        .with(internal_tracing_layer)
        .init();
    let build_dir = config.build_dir.clone();
    if let Some(Command::Doctor { json }) = &cli.command {
        return doctor(&build_dir, *json).await;
    }
    match &cli.command {
        Some(Command::Inspect) => {
            return inspect_workspace(config.backend.clone(), build_dir).await;
        }
        Some(Command::Profile) => {
            return inspect_project_profile(config.backend.clone(), build_dir).await;
        }
        Some(Command::Recipes) => return print_recipes(config.backend.clone(), build_dir).await,
        Some(Command::Layers) => return print_layers(config.backend.clone(), build_dir).await,
        Some(Command::Config { name }) => {
            return print_variable(config.backend.clone(), build_dir, name).await;
        }
        Some(Command::Doctor { .. })
        | Some(Command::Build { .. })
        | Some(Command::Attach)
        | None => {}
        Some(Command::Sessions | Command::Session { .. }) => unreachable!(),
        Some(Command::Daemon { .. }) => unreachable!("daemon command handled before config"),
    }
    let targets = match &cli.command {
        Some(Command::Build { targets }) => targets.clone(),
        _ if !cli.targets.is_empty() => cli.targets.clone(),
        _ => config
            .default_target
            .clone()
            .or(session.last_target.clone())
            .into_iter()
            .collect(),
    };
    if cli.headless {
        return headless(
            config.backend,
            build_dir,
            targets,
            config.log_entries,
            config.log_bytes,
        )
        .await;
    }
    tui(config, targets, session, internal_tracing_capture).await
}

mod cli_arguments;
use cli_arguments::*;
mod configuration;
use configuration::*;
mod session_store;
use session_store::*;
mod terminal_lifecycle;
use terminal_lifecycle::*;
mod host_telemetry;
use host_telemetry::*;
mod project_profile;
pub use project_profile::generate_project_profile;
use project_profile::*;
mod workspace_commands;
use workspace_commands::*;
mod config_query;
use config_query::*;
mod doctor_report;
use doctor_report::*;
mod doctor_render;
use doctor_render::*;
mod doctor_command;
use doctor_command::*;
mod daemon_commands;
use daemon_commands::*;
mod daemon_scheduling;
use daemon_scheduling::*;
mod daemon_startup;
use daemon_startup::*;
mod daemon_server;
use daemon_server::*;
mod daemon_publish_build;
use daemon_publish_build::*;
mod daemon_publish_pty;
use daemon_publish_pty::*;
mod daemon_publish_raw;
use daemon_publish_raw::*;
mod daemon_publish_devtool;
use daemon_publish_devtool::*;
mod daemon_publish_sdk;
use daemon_publish_sdk::*;
mod daemon_publish_qemu;
use daemon_publish_qemu::*;
mod daemon_publish_wic;
use daemon_publish_wic::*;
mod daemon_publish_test;
use daemon_publish_test::*;
mod daemon_publish_qa;
use daemon_publish_qa::*;
mod daemon_publish_security;
use daemon_publish_security::*;
mod daemon_publish_maintenance;
use daemon_publish_maintenance::*;
mod daemon_identity;
use daemon_identity::*;
mod daemon_service;
use daemon_service::*;
mod headless_build;
use headless_build::*;
mod backend_startup;
use backend_startup::*;
mod build_operations;
use build_operations::*;
mod test_build;
use test_build::*;
mod security_build;
use security_build::*;
mod qa_build;
use qa_build::*;
mod devtool_jobs;
use devtool_jobs::*;
mod sdk_operations;
use sdk_operations::*;
mod sdk_jobs;
use sdk_jobs::*;
mod test_coordinator;
use test_coordinator::*;
mod security_coordinator;
use security_coordinator::*;
mod security_inspection;
use security_inspection::*;
mod qa_coordinator;
use qa_coordinator::*;
mod qa_capabilities;
use qa_capabilities::*;
mod qa_reports;
use qa_reports::*;
mod path_validation;
use path_validation::*;
mod qa_effects;
use qa_effects::*;
mod terminal_launcher;
use terminal_launcher::*;
mod maintenance_effects;
use maintenance_effects::*;
mod test_capabilities;
use test_capabilities::*;
mod sdk_build;
use sdk_build::*;
mod qemu_jobs;
use qemu_jobs::*;
mod wic_jobs;
use wic_jobs::*;
mod signature_operations;
use signature_operations::*;
mod package_operations;
use package_operations::*;
mod image_operations;
use image_operations::*;
mod workspace_effects;
use workspace_effects::*;
mod wic_inspection;
use wic_inspection::*;
mod rootfs_operations;
use rootfs_operations::*;
mod content_search;
use content_search::*;
mod external_editor;
use external_editor::*;
mod devtool_completion;
use devtool_completion::*;
mod config_inspection;
use config_inspection::*;
mod workspace_editor;
use workspace_editor::*;
mod kernel_inspection;
use kernel_inspection::*;
mod firmware_inspection;
use firmware_inspection::*;
mod layer_browser;
use layer_browser::*;
mod recipe_editor;
use recipe_editor::*;
mod config_writer;
use config_writer::*;
mod workspace_refresh;
use workspace_refresh::*;
mod input_routing;
use input_routing::*;
mod client_origin;
use client_origin::*;
mod interactive_runtime;
use interactive_runtime::*;
mod termination;
use termination::*;

#[cfg(test)]
#[path = "tests/firmware_workbench/mod.rs"]
mod firmware_workbench_tests;

#[cfg(test)]
#[path = "tests/cli/mod.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/m68_focus/mod.rs"]
mod m68_focus_tests;
