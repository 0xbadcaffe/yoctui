use super::*;

#[test]
fn maintenance_release_archive_workspace_rejects_invalid_notes_context_and_bounds() {
    let mut state = MaintenanceState {
        view: MaintenanceView::Release,
        ..MaintenanceState::default()
    };
    assert_eq!(
        update_maintenance(&mut state, MaintenanceAction::OpenGitArchiveForm),
        MaintenanceTransition::none()
    );
    let mut draft = MaintenanceGitArchiveDraft {
        data_dir: "/release/data".into(),
        git_dir: "/release/archive.git".into(),
        notes: "missing-equals".into(),
        ..MaintenanceGitArchiveDraft::default()
    };
    let invalid = update_maintenance(
        &mut state,
        MaintenanceAction::ConfirmGitArchiveForm(Box::new(draft.clone())),
    );
    assert!(matches!(
        invalid.dialog,
        MaintenanceDialogUpdate::Open(dialog)
            if matches!(*dialog, MaintenanceDialog::GitArchiveForm(ref draft) if draft.validation.as_deref().is_some_and(|message| message.contains("reference=")))
    ));
    draft.notes = "x".repeat(MAX_MAINTENANCE_TEXT_BYTES + 1);
    assert_eq!(
        update_maintenance(
            &mut state,
            MaintenanceAction::UpdateGitArchiveForm(Box::new(draft)),
        ),
        MaintenanceTransition::none()
    );
}
