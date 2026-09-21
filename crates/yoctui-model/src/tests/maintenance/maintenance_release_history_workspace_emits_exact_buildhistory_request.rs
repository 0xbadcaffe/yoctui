use super::*;

#[test]
fn maintenance_release_history_workspace_emits_exact_buildhistory_request() {
    let mut state = ready_state();
    state.view = MaintenanceView::Release;
    let transition = update_maintenance(&mut state, MaintenanceAction::OpenBuildHistoryForm);
    let MaintenanceDialogUpdate::Open(dialog) = transition.dialog else {
        panic!("build-history form did not open");
    };
    let MaintenanceDialog::BuildHistoryToml { editor, .. } = *dialog else {
        panic!("wrong build-history dialog");
    };
    assert!(
        editor
            .text
            .contains("# Repository (read-only): /build/buildhistory")
    );
    let valid = update_maintenance(
            &mut state,
            MaintenanceAction::ConfirmBuildHistoryToml("from_revision = \"HEAD~2\"\nto_revision = \"HEAD\"\nreport_version = true\nreport_all = false\nsignatures = true\nsignature_diff = true\nexclude_paths = \"images/*, packages/*, images/*\"\nno_colour = true\n".into()),
        );
    assert!(matches!(
        valid.effect,
        Some(MaintenanceEffect::PreviewBuildHistoryComparison {
            capability_request: 1,
            request: BuildComparisonRequest {
                from_revision: Some(from),
                to_revision: Some(to),
                signatures: true,
                signature_diff: true,
                no_colour: true,
                exclude_paths,
                ..
            },
        }) if from == "HEAD~2"
            && to == "HEAD"
            && exclude_paths == vec!["images/*", "packages/*"]
    ));
    assert_eq!(valid.dialog, MaintenanceDialogUpdate::Close);
}
