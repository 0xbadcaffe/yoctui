use super::*;
use crate::{Action, App, Dialog, Effect, FocusTarget, Screen, update};

fn recipe() -> RecipeIdentity {
    RecipeIdentity {
        name: "busybox".into(),
        file: "/layers/meta/recipes-core/busybox/busybox_1.0.bb".into(),
    }
}

fn capability() -> SecurityCapabilitySnapshot {
    SecurityCapabilitySnapshot::new(
        Some("6.0".into()),
        "/build".into(),
        SecurityScope::Recipe(recipe()),
        vec![SecurityScope::Recipe(recipe())],
        Some("cve_check".into()),
        Some("create_recipe_sbom".into()),
        None,
        false,
        Some(SecurityMapperCapability {
            executable: "/tools/cve-check-map-pkgs".into(),
            arguments: vec!["/build/tmp/log/cve".into()],
        }),
        vec!["/build/tmp/log/cve".into()],
        vec!["/build/tmp/deploy/spdx".into()],
        vec![],
    )
    .unwrap()
}

fn identity(path: &str, fingerprint: &str) -> SecurityReportIdentity {
    SecurityReportIdentity::new(path.into(), 12, SystemTime::UNIX_EPOCH, fingerprint.into())
        .unwrap()
}

fn finding(status: CveStatus) -> CveFinding {
    CveFinding {
        identity: CveFindingIdentity::new(
            "CVE-2026-0001".into(),
            "busybox".into(),
            Some("busybox".into()),
        )
        .unwrap(),
        status,
        product: Some("busybox".into()),
        version: Some("1.0".into()),
        severity: Some("HIGH".into()),
        score: Some("8.1".into()),
        vector: None,
        advisory_url: Some("https://example.invalid/CVE-2026-0001".into()),
        summary: Some("bounds check".into()),
        mapping: vec![],
    }
}

mod security_workflow_previews_capability_supplied_current_and_legacy_tasks;

mod security_workflow_normalizes_reports_and_rejects_stale_generations;

mod security_workflow_correlates_session_terminal_and_refresh_states;

mod security_workflow_app_navigation_dialog_and_effects_are_typed;

mod security_import_popup_selects_root_and_keeps_validation_in_dialog;
