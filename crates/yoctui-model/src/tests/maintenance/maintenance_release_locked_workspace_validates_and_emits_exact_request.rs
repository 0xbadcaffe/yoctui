use super::*;

#[test]
fn maintenance_release_locked_workspace_validates_and_emits_exact_request() {
    let mut state = ready_state();
    state.view = MaintenanceView::Release;
    let transition = update_maintenance(&mut state, MaintenanceAction::OpenLockedCacheForm);
    let MaintenanceDialogUpdate::Open(dialog) = transition.dialog else {
        panic!("locked-cache form did not open");
    };
    let MaintenanceDialog::LockedCacheToml { editor, .. } = *dialog else {
        panic!("wrong locked-cache dialog");
    };
    assert_eq!(editor.selected_text(), Some(""));
    assert!(editor.text.contains("# Native LSB (read-only): ubuntu"));
    let invalid = update_maintenance(
        &mut state,
        MaintenanceAction::ConfirmLockedCacheToml(editor.text),
    );
    assert!(matches!(
        invalid.dialog,
        MaintenanceDialogUpdate::Open(dialog)
            if matches!(*dialog, MaintenanceDialog::LockedCacheToml { validation_error: Some(_), .. })
    ));

    let valid = update_maintenance(
            &mut state,
            MaintenanceAction::ConfirmLockedCacheToml(
                "locked_signatures = \"/build/conf/locked-sigs.inc\"\ninput_cache = \"/cache/input\"\noutput_cache = \"/cache/release\"\nfilter = \"/build/conf/locked-filter.inc\"\n".into(),
            ),
        );
    assert!(matches!(
        valid.effect,
        Some(MaintenanceEffect::PreviewLockedSignatureCache {
            capability_request: 1,
            request: LockedSignatureCacheRequest {
                native_lsb,
                filter: Some(filter),
                ..
            },
        }) if native_lsb == "ubuntu" && filter == Path::new("/build/conf/locked-filter.inc")
    ));
    assert_eq!(valid.dialog, MaintenanceDialogUpdate::Close);
}
