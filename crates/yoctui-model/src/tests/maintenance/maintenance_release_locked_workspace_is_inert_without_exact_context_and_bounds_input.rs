use super::*;

#[test]
fn maintenance_release_locked_workspace_is_inert_without_exact_context_and_bounds_input() {
    let mut state = MaintenanceState {
        view: MaintenanceView::Release,
        ..MaintenanceState::default()
    };
    assert_eq!(
        update_maintenance(&mut state, MaintenanceAction::OpenLockedCacheForm),
        MaintenanceTransition::none()
    );

    let mut draft = MaintenanceLockedCacheDraft::from_metadata(&MaintenanceMetadata {
        native_lsb: Some("ubuntu".into()),
        ..MaintenanceMetadata::default()
    })
    .unwrap();
    draft.locked_signatures = "x".repeat(MAX_MAINTENANCE_TEXT_BYTES + 1);
    assert!(!draft.is_bounded());
    assert_eq!(
        update_maintenance(
            &mut state,
            MaintenanceAction::UpdateLockedCacheForm(Box::new(draft)),
        ),
        MaintenanceTransition::none()
    );
}
