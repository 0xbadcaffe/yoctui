use super::*;

#[test]
fn maintenance_sstate_workspace_forms_validate_and_emit_typed_preview_requests() {
    let mut state = ready_state();
    let transition = update_maintenance(&mut state, MaintenanceAction::OpenReadinessForm);
    let MaintenanceDialogUpdate::Open(dialog) = transition.dialog else {
        panic!("readiness form did not open");
    };
    let MaintenanceDialog::ReadinessToml { editor, .. } = *dialog else {
        panic!("wrong readiness dialog");
    };
    assert_eq!(editor.selected_text(), Some(""));
    let invalid = update_maintenance(
        &mut state,
        MaintenanceAction::ConfirmReadinessToml(editor.text.clone()),
    );
    assert!(matches!(
        invalid.dialog,
        MaintenanceDialogUpdate::Open(dialog)
            if matches!(*dialog, MaintenanceDialog::ReadinessToml { validation_error: Some(_), .. })
    ));
    let document = r#"# exact sstate readiness request
targets = "core-image-minimal busybox"
mode = "isolated_tmpdir"
output = ""
log = ""
timeout = 45
"#;
    let valid = update_maintenance(
        &mut state,
        MaintenanceAction::ConfirmReadinessToml(document.into()),
    );
    assert!(matches!(
        valid.effect,
        Some(MaintenanceEffect::PreviewReadiness {
            capability_request: 1,
            request: SstateReadinessRequest {
                timeout_seconds: 45,
                ..
            },
        })
    ));
    assert_eq!(valid.dialog, MaintenanceDialogUpdate::Close);

    let cleanup = update_maintenance(&mut state, MaintenanceAction::OpenCleanupForm);
    let MaintenanceDialogUpdate::Open(dialog) = cleanup.dialog else {
        panic!("cleanup form did not open");
    };
    let MaintenanceDialog::CleanupToml { editor, .. } = *dialog else {
        panic!("wrong cleanup dialog");
    };
    assert_eq!(editor.selected_text(), Some("true"));
    assert!(editor.text.contains("# Cache (read-only): /cache"));
    let invalid = update_maintenance(
        &mut state,
        MaintenanceAction::ConfirmCleanupToml(
            "duplicates = false\norphans = false\nunreferenced_by_stamps = false\njobs = 1\n"
                .into(),
        ),
    );
    assert!(matches!(
        invalid.dialog,
        MaintenanceDialogUpdate::Open(dialog)
            if matches!(*dialog, MaintenanceDialog::CleanupToml { validation_error: Some(_), .. })
    ));
    let valid = update_maintenance(
        &mut state,
        MaintenanceAction::ConfirmCleanupToml(
            "duplicates = false\norphans = true\nunreferenced_by_stamps = false\njobs = 3\n".into(),
        ),
    );
    assert!(matches!(
        valid.effect,
        Some(MaintenanceEffect::PreviewCleanup {
            capability_request: 1,
            request: SstateCleanupRequest { jobs: 3, ref cache_dir, ref stamps_dirs, .. },
        }) if cache_dir == Path::new("/cache") && stamps_dirs == &[PathBuf::from("/build/tmp/stamps")]
    ));
}
