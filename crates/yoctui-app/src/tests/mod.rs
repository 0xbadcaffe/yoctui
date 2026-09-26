//! Shared fixtures and regression modules.

use super::*;

pub(crate) fn compatibility_workspace_authority(
    generation: u64,
) -> yoctui_model::DaemonCompatibilitySnapshot {
    let environment = yoctui_model::YoctoEnvironmentIdentity {
        build_directory: yoctui_model::AuthoritativeValue::detected(
            "/work/poky/build".into(),
            yoctui_model::IdentityAuthority::InitializedEnvironment,
        ),
        bitbake_version: yoctui_model::AuthoritativeValue::detected(
            "2.18.0".into(),
            yoctui_model::IdentityAuthority::BitBakeVersionProbe,
        ),
        ..yoctui_model::YoctoEnvironmentIdentity::default()
    };
    let available = |id| yoctui_model::CapabilityRecord {
        id,
        state: yoctui_model::CapabilityState::Available,
        evidence: vec![yoctui_model::CapabilityEvidence {
            kind: yoctui_model::CapabilityEvidenceKind::DirectProbe,
            outcome: yoctui_model::CapabilityEvidenceOutcome::Positive,
            subject: id.as_str().into(),
            detail: "Verified by the compatibility workspace app fixture.".into(),
            argv: Vec::new(),
        }],
    };
    yoctui_model::DaemonCompatibilitySnapshot {
        snapshot: yoctui_model::CapabilitySnapshot {
            generation,
            environment,
            capabilities: vec![
                available(yoctui_model::CapabilityId::BitBakeBuild),
                available(yoctui_model::CapabilityId::PkgDataGenerated),
                available(yoctui_model::CapabilityId::PkgDataListPackages),
                yoctui_model::CapabilityRecord {
                    id: yoctui_model::CapabilityId::DevtoolUpgrade,
                    state: yoctui_model::CapabilityState::Unavailable {
                        reason: yoctui_model::CapabilityReason::new(
                            "probe.subcommand_absent",
                            "Current Devtool does not expose the upgrade subcommand.",
                            Some("devtool upgrade".into()),
                        )
                        .unwrap(),
                    },
                    evidence: vec![yoctui_model::CapabilityEvidence {
                        kind: yoctui_model::CapabilityEvidenceKind::DirectProbe,
                        outcome: yoctui_model::CapabilityEvidenceOutcome::Negative,
                        subject: "devtool upgrade".into(),
                        detail: "The compatibility workspace app fixture omits upgrade.".into(),
                        argv: vec!["devtool".into(), "--help".into()],
                    }],
                },
            ],
        },
        implementations: std::collections::BTreeMap::from([
            (
                yoctui_model::CapabilityId::BitBakeBuild,
                yoctui_model::CapabilityImplementation {
                    id: "bitbake.build.command".into(),
                    kind: yoctui_model::CapabilityImplementationKind::Command,
                },
            ),
            (
                yoctui_model::CapabilityId::PkgDataGenerated,
                yoctui_model::CapabilityImplementation {
                    id: "pkgdata.generated.metadata".into(),
                    kind: yoctui_model::CapabilityImplementationKind::MetadataTask,
                },
            ),
            (
                yoctui_model::CapabilityId::PkgDataListPackages,
                yoctui_model::CapabilityImplementation {
                    id: "pkgdata.list-packages.command".into(),
                    kind: yoctui_model::CapabilityImplementationKind::Command,
                },
            ),
        ]),
    }
}

pub(crate) fn compatibility_workspace_daemon_snapshot(
    authority: &yoctui_model::DaemonCompatibilitySnapshot,
) -> yoctui_protocol::daemon::DaemonSnapshot {
    let state = yoctui_model::DaemonGlobalState::new(
        yoctui_model::DaemonModelInstanceId([0x18; 16]),
        123,
        "compatibility-workspace".into(),
        yoctui_model::DaemonStateLimits::default(),
    )
    .unwrap();
    let mut snapshot = daemon_protocol_snapshot(&state);
    snapshot.compatibility = Some(daemon_compatibility_protocol(authority));
    snapshot
}
use std::{path::PathBuf, time::Duration};
use yoctui_model::{
    App, BackgroundJobStatus, BuildStatus, DependencyEdge, DependencyEdgeKind, DependencyGraph,
    DependencyGraphState, DependencyNodeId, ImageArtifact, ImageArtifactField,
    ImageArtifactIdentity, ImageArtifactInventory, ImageArtifactKind, ImageArtifactRequest,
    PackageDetail, PackageDetailRequest, PackageField, PackageIdentity, PackageInventoryRequest,
    PackageSummary, SignatureComparisonRequest, SignatureDifference, SignatureDifferenceCategory,
    SignatureIdentity, SignatureRecord, SignatureTarget, update,
};

pub(crate) fn apply_actions(app: &mut App, actions: Vec<Action>) {
    for action in actions {
        let _ = update(app, action);
    }
}

pub(crate) fn request() -> BuildRequest {
    BuildRequest {
        targets: vec!["core-image-minimal".into()],
        task: None,
        force: false,
    }
}

pub(crate) fn maintenance_preview() -> yoctui_model::MaintenanceOperationPreview {
    yoctui_model::MaintenanceOperationPreview::new(
        7,
        1,
        yoctui_model::MaintenanceOperation::SstateReadiness(
            yoctui_model::SstateReadinessRequest::new(
                vec!["core-image-minimal".into()],
                yoctui_model::SstateReadinessMode::IsolatedTmpdir,
                None,
                None,
                60,
            )
            .unwrap(),
        ),
        vec![
            "0: /tools/oe-check-sstate".into(),
            "1: core-image-minimal".into(),
        ],
        vec![],
    )
    .unwrap()
}

pub(crate) fn raw_selector_command(
    kind: yoctui_model::RawParameterKind,
) -> (yoctui_model::RawCommand, yoctui_model::RawParameterId) {
    let command = yoctui_model::RawCatalog::builtin()
        .commands
        .into_iter()
        .find(|command| {
            matches!(
                command.execution,
                yoctui_model::RawExecutionPolicy::Executable { .. }
            ) && command
                .parameters
                .iter()
                .any(|parameter| parameter.kind == kind)
        })
        .unwrap();
    let parameter = command
        .parameters
        .iter()
        .find(|parameter| parameter.kind == kind)
        .unwrap()
        .id
        .clone();
    (command, parameter)
}

pub(crate) fn raw_execution_state_fixture() -> yoctui_model::RawExecutionState {
    let request = yoctui_model::RawConfirmedExecutionRequest {
        id: yoctui_model::RawRequestId::new("raw-request:app-1").unwrap(),
        catalog_version: 1,
        command: yoctui_model::RawCommandId::new("build.target").unwrap(),
        parameters: std::collections::BTreeMap::from([(
            yoctui_model::RawParameterId::new("target").unwrap(),
            yoctui_model::RawParameterValue::Target("core-image-minimal".into()),
        )]),
        additional_arguments: vec!["--dry-run".into()],
        interaction: yoctui_model::RawInteractionMode::NoninteractiveJob,
        safety: yoctui_model::RawSafetyClass::Build,
        capability_generation: 7,
        build_directory: std::path::PathBuf::from("/work/build"),
        preview_digest: yoctui_model::RawPreviewDigest([7; 32]),
    };
    yoctui_model::RawExecutionState::queued(
        request,
        yoctui_model::RawStreamId::new("raw-stream:app-stdout").unwrap(),
        yoctui_model::RawStreamId::new("raw-stream:app-stderr").unwrap(),
        10,
        yoctui_model::RawEventCursor::default(),
    )
    .unwrap()
}

pub(crate) fn apply_raw_execution_fixture(
    state: &mut yoctui_model::RawExecutionState,
    kind: yoctui_model::RawExecutionEventKind,
) -> yoctui_model::RawExecutionEvent {
    let event = yoctui_model::RawExecutionEvent {
        request_id: state.request.id.clone(),
        sequence: state.cursor.sequence + 1,
        generation: state.cursor.generation + 1,
        kind,
    };
    yoctui_model::reduce_raw_execution(state, event.clone()).unwrap();
    event
}
mod compatibility_dynamic_app_converts_installs_updates_and_invalidates_authority;
mod daemon_recovery_restores_metadata_without_claiming_live_bitbake_or_profile;
mod dependency_graph_maps_typed_navigation_filter_topology_and_open_actions;
mod devtool_editor_git;
mod devtool_editor_search;
mod devtool_editor_viewport;
mod devtool_workspace;
mod image_artifact_adapter_response_crosses_the_app_boundary_as_typed_action;
mod mouse_runtime_routes_dialog_and_terminal_session_clicks;
mod navigator_mouse_and_keyboard_share_typed_routing;
mod preferences_disable_mouse_and_route_preview_reset_keys_exactly;
mod qemu_model_normalizes_typed_runner_events_without_parsing_output;
mod raw_mode_app_routes_only_the_selected_additional_field_to_shared_editor;
mod snapshot_timing_matches_live_batch_replacement_and_terminal_replay;
mod test_results_junit_events_keep_terminal_outcomes_distinct;
mod yocto_utility;

use super::daemon_client::apply_daemon_build_event;
use super::keyboard::DEFAULT_COLLECTION_PAGE_ROWS;
use super::mouse::{MouseRect, task_row_click};
