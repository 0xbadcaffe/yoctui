use super::*;
use crate::{Action, App, Dialog, Effect, FocusTarget, update};

fn scope(name: &str) -> QaScope {
    QaScope::new(RecipeIdentity {
        name: name.into(),
        file: format!("/layers/meta/recipes/{name}/{name}_1.0.bb").into(),
    })
    .unwrap()
}

fn check(id: &str, family: QaCheckFamily, scope: QaScope, task: Option<&str>) -> QaCheckCapability {
    let availability = task.map_or_else(
        || QaCheckAvailability::Disabled("task not reported".into()),
        |_| QaCheckAvailability::Available,
    );
    QaCheckCapability::new(
        QaCheckId::new(id.into()).unwrap(),
        family,
        format!("{family:?}"),
        scope,
        task.map(str::to_owned),
        vec!["/build/tmp/log/qa".into()],
        availability,
        vec![],
    )
    .unwrap()
}

fn capability() -> QaCapabilitySnapshot {
    let kernel = scope("linux-yocto");
    let busybox = scope("busybox");
    QaCapabilitySnapshot::new(
        Some("6.0".into()),
        "/build".into(),
        kernel.clone(),
        vec![kernel.clone(), busybox.clone()],
        vec![
            check(
                "kernel-config",
                QaCheckFamily::KernelConfiguration,
                kernel.clone(),
                Some("kernel_configcheck"),
            ),
            check(
                "uri-check",
                QaCheckFamily::UriFetch,
                kernel.clone(),
                Some("checkuri"),
            ),
            check("patch-check", QaCheckFamily::Patch, kernel.clone(), None),
            check(
                "license-check",
                QaCheckFamily::License,
                busybox.clone(),
                Some("populate_lic"),
            ),
            check(
                "recipe-qa",
                QaCheckFamily::RecipePackage,
                busybox,
                Some("package_qa"),
            ),
        ],
        vec![],
    )
    .unwrap()
}

fn load(state: &mut QaState) {
    let _ = update_qa(state, QaAction::CapabilityLoaded(capability()));
}

fn begin(state: &mut QaState) -> QaOperationPreview {
    let transition = update_qa(state, QaAction::BeginSelectedCheck);
    let QaDialogUpdate::Open(dialog) = transition.dialog else {
        panic!("expected operation preview")
    };
    let QaDialog::Operation(preview) = *dialog else {
        panic!("expected operation preview")
    };
    preview
}

fn start(state: &mut QaState) -> QaSessionId {
    let preview = begin(state);
    let transition = update_qa(state, QaAction::ConfirmOperation(preview));
    let Some(QaEffect::StartBuild { session, .. }) = transition.effect else {
        panic!("expected managed build")
    };
    session
}

fn report_identity(check: QaCheckId) -> QaReportIdentity {
    QaReportIdentity::new(
        "/build/tmp/log/qa/report.json".into(),
        100,
        SystemTime::UNIX_EPOCH,
        "abc123".into(),
        QaReportFormat::Json,
        Some(check),
        Some(QaFindingScope::Recipe(scope("linux-yocto"))),
    )
    .unwrap()
}

fn finding(status: QaFindingStatus, fingerprint: &str) -> QaFinding {
    QaFinding {
        identity: QaFindingIdentity::new(
            QaCheckId::new("kernel-config".into()).unwrap(),
            fingerprint.into(),
        )
        .unwrap(),
        status,
        severity: Some("warning".into()),
        message: "CONFIG_DEVMEM differs from policy".into(),
        scope: QaFindingScope::Recipe(scope("linux-yocto")),
        task: Some("kernel_configcheck".into()),
        test_name: None,
        source: Some(
            QaSourceLocation::new("/layers/meta/cfg/policy.cfg".into(), Some(7), None).unwrap(),
        ),
        rule: Some("CONFIG_DEVMEM".into()),
        suggestion: Some("disable the option".into()),
        metadata: vec![],
    }
}

fn report(findings: Vec<QaFinding>) -> QaReport {
    QaReport {
        identity: report_identity(QaCheckId::new("kernel-config".into()).unwrap()),
        findings,
        metadata: vec![],
        limitations: vec![],
    }
}

mod qa_check_workflow_capability_catalog_never_guesses_tasks;

mod qa_check_workflow_preview_is_indexed_exact_and_stale_safe;

mod qa_check_workflow_session_lifecycle_output_bounds_and_cancellation_are_typed;

mod qa_check_workflow_keeps_failure_cancel_timeout_and_loss_distinct;

mod qa_check_workflow_session_history_is_bounded;

mod qa_check_workflow_success_scans_exact_results_and_empty_is_honest;

mod qa_check_workflow_report_generations_bounds_partial_and_terminal_states;

mod qa_check_workflow_search_filter_selection_drill_and_exact_opens;

mod qa_check_workflow_scope_cycle_preserves_exact_provider_identity;

fn layer(name: &str) -> QaLayerIdentity {
    QaLayerIdentity::new(name.into(), format!("/layers/{name}").into()).unwrap()
}

fn layer_executable() -> QaExecutableIdentity {
    QaExecutableIdentity::new(
        "/poky/scripts/yocto-check-layer".into(),
        1_024,
        SystemTime::UNIX_EPOCH,
    )
    .unwrap()
}

fn layer_row(name: &str, available: bool) -> QaConfiguredLayerCapability {
    let identity = layer(name);
    let run = if available {
        QaLayerRunCapability::Available {
            executable: layer_executable(),
            arguments: vec![
                "--layer".into(),
                identity.root.to_string_lossy().into_owned(),
            ],
            report_roots: vec!["/build/tmp/log/qa-layer".into()],
        }
    } else {
        QaLayerRunCapability::Disabled("yocto-check-layer is unavailable".into())
    };
    QaConfiguredLayerCapability::new(
        QaCheckId::new("yocto-check-layer".into()).unwrap(),
        identity,
        vec!["whinlatter".into()],
        run,
        vec![],
    )
    .unwrap()
}

fn layer_capability() -> QaLayerCapabilitySnapshot {
    QaLayerCapabilitySnapshot::new(
        Some("6.0".into()),
        "/build".into(),
        layer("meta"),
        vec![layer_row("meta", true), layer_row("meta-custom", false)],
        vec![],
    )
    .unwrap()
}

fn load_layer(state: &mut QaState) {
    state.view = QaView::LayerQa;
    let _ = update_qa(state, QaAction::LayerCapabilityLoaded(layer_capability()));
}

fn begin_layer(state: &mut QaState) -> QaLayerOperationPreview {
    let transition = update_qa(state, QaAction::BeginSelectedLayerCheck);
    let QaDialogUpdate::Open(dialog) = transition.dialog else {
        panic!("expected layer QA preview")
    };
    let QaDialog::LayerOperation(preview) = *dialog else {
        panic!("expected layer QA preview")
    };
    preview
}

fn start_layer(state: &mut QaState) -> QaLayerSessionId {
    let preview = begin_layer(state);
    let transition = update_qa(state, QaAction::ConfirmLayerOperation(preview));
    let Some(QaEffect::StartLayerCheck { session, .. }) = transition.effect else {
        panic!("expected layer QA start")
    };
    session
}

fn layer_finding(status: QaFindingStatus, fingerprint: &str) -> QaFinding {
    QaFinding {
        identity: QaFindingIdentity::new(
            QaCheckId::new("yocto-check-layer".into()).unwrap(),
            fingerprint.into(),
        )
        .unwrap(),
        status,
        severity: Some("warning".into()),
        message: "layer compatibility declaration is incomplete".into(),
        scope: QaFindingScope::Layer(layer("meta")),
        task: None,
        test_name: Some("LayerCompatibility".into()),
        source: Some(
            QaSourceLocation::new("/layers/meta/conf/layer.conf".into(), Some(12), None).unwrap(),
        ),
        rule: Some("LAYERSERIES_COMPAT".into()),
        suggestion: Some("declare the active release series".into()),
        metadata: vec![],
    }
}

fn layer_report(findings: Vec<QaFinding>) -> QaReport {
    let check = QaCheckId::new("yocto-check-layer".into()).unwrap();
    QaReport {
        identity: QaReportIdentity::new(
            "/build/tmp/log/qa-layer/report.json".into(),
            512,
            SystemTime::UNIX_EPOCH,
            "layerreport".into(),
            QaReportFormat::Json,
            Some(check),
            Some(QaFindingScope::Layer(layer("meta"))),
        )
        .unwrap(),
        findings,
        metadata: vec![],
        limitations: vec![],
    }
}

mod qa_layer_workflow_keeps_configured_disabled_layers_and_rejects_invalid_identities;

mod qa_layer_workflow_preview_is_exact_indexed_and_stale_safe;

mod qa_layer_workflow_lifecycle_output_cancellation_and_terminal_states_are_distinct;

mod qa_layer_workflow_reports_counts_filters_drill_and_exact_opens;

mod qa_layer_workflow_native_session_is_independent_from_managed_build_session;

mod qa_layer_workflow_history_is_bounded_and_dialog_focus_is_trapped;

mod qa_check_workflow_app_boundary_traps_dialog_focus_and_maps_typed_effects;
