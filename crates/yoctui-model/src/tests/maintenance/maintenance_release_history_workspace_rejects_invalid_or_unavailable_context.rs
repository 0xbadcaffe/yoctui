use super::*;

#[test]
fn maintenance_release_history_workspace_rejects_invalid_or_unavailable_context() {
    let mut state = MaintenanceState {
        view: MaintenanceView::Release,
        ..MaintenanceState::default()
    };
    assert_eq!(
        update_maintenance(&mut state, MaintenanceAction::OpenBuildHistoryForm),
        MaintenanceTransition::none()
    );

    let mut draft = MaintenanceBuildHistoryDraft::from_metadata(&MaintenanceMetadata {
        buildhistory_dir: Some(PathBuf::from("relative/buildhistory")),
        ..MaintenanceMetadata::default()
    })
    .unwrap();
    let invalid = update_maintenance(
        &mut state,
        MaintenanceAction::ConfirmBuildHistoryForm(Box::new(draft.clone())),
    );
    assert!(matches!(
        invalid.dialog,
        MaintenanceDialogUpdate::Open(dialog)
            if matches!(*dialog, MaintenanceDialog::BuildHistoryForm(ref draft) if draft.validation.is_some())
    ));
    draft.exclude_paths = "x".repeat(MAX_MAINTENANCE_TEXT_BYTES + 1);
    assert_eq!(
        update_maintenance(
            &mut state,
            MaintenanceAction::UpdateBuildHistoryForm(Box::new(draft)),
        ),
        MaintenanceTransition::none()
    );
}
