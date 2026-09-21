use super::*;
use std::time::{Duration, UNIX_EPOCH};

fn identity(path: &str) -> MaintenanceFileIdentity {
    MaintenanceFileIdentity::new(PathBuf::from(path), 10, UNIX_EPOCH).unwrap()
}

fn capability() -> MaintenanceCapabilitySnapshot {
    MaintenanceCapabilitySnapshot::new(
        MaintenanceMetadata::new(MaintenanceMetadata {
            build_dir: Some("/build".into()),
            sstate_dir: Some("/cache".into()),
            stamps_dirs: vec!["/build/tmp/stamps".into()],
            buildhistory_dir: Some("/build/buildhistory".into()),
            native_lsb: Some("ubuntu".into()),
            machine: Some("qemux86-64".into()),
            prserv_host: Some("localhost:8585".into()),
            ..MaintenanceMetadata::default()
        })
        .unwrap(),
        [
            MaintenanceTool::OeCheckSstate,
            MaintenanceTool::SstateCacheManagement,
            MaintenanceTool::PrServiceTool,
            MaintenanceTool::LockedSignatureCache,
            MaintenanceTool::BuildHistoryDiff,
            MaintenanceTool::BuildCompare,
            MaintenanceTool::GitArchive,
        ]
        .into_iter()
        .map(|tool| MaintenanceToolCapability::Available {
            tool,
            executable: identity(&format!("/tools/{tool:?}")),
            interface: MaintenanceToolInterface::Native,
        })
        .collect(),
        vec![],
    )
    .unwrap()
}

fn ready_state() -> MaintenanceState {
    let mut state = MaintenanceState::default();
    let transition = update_maintenance(&mut state, MaintenanceAction::InspectCapability);
    assert_eq!(
        transition.effect,
        Some(MaintenanceEffect::InspectCapability { request: 1 })
    );
    update_maintenance(
        &mut state,
        MaintenanceAction::CapabilityLoaded {
            request: 1,
            snapshot: capability(),
            partial: false,
        },
    );
    state
}

fn readiness_preview(id: u64) -> MaintenanceOperationPreview {
    MaintenanceOperationPreview::new(
        id,
        1,
        MaintenanceOperation::SstateReadiness(
            SstateReadinessRequest::new(
                vec!["core-image-minimal".into()],
                SstateReadinessMode::IsolatedTmpdir,
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

mod maintenance_workflow_preserves_selection_across_fixed_views;

mod maintenance_workflow_rejects_stale_capability_and_preview;

mod maintenance_workflow_requires_exact_cleanup_phrase;

mod maintenance_workflow_covers_success_failure_timeout_cancel_and_loss;

mod maintenance_workflow_bounds_output_and_replaces_only_successful_evidence;

mod maintenance_workflow_routes_existing_owned_destinations;

mod maintenance_service_model_retains_typed_endpoint_process_and_pr_context;

mod maintenance_release_model_retains_no_colour_and_separate_archive_push_intent;

mod maintenance_integration_snapshot_validates_state_and_bounds_typed_evidence;

mod maintenance_workflow_correlates_replaceable_integration_diagnostics;

mod maintenance_sstate_workspace_forms_validate_and_emit_typed_preview_requests;

mod maintenance_sstate_workspace_forms_stay_inert_without_exact_capability;

mod maintenance_service_workspace_forms_keep_operation_and_context_typed;

mod maintenance_service_workspace_form_is_inert_without_capability_or_context;

mod maintenance_release_locked_workspace_validates_and_emits_exact_request;

mod maintenance_release_locked_workspace_is_inert_without_exact_context_and_bounds_input;

mod maintenance_release_history_workspace_emits_exact_buildhistory_request;

mod maintenance_release_history_workspace_rejects_invalid_or_unavailable_context;

mod maintenance_release_archive_workspace_preserves_local_and_push_intent;

mod maintenance_release_archive_workspace_rejects_invalid_notes_context_and_bounds;
