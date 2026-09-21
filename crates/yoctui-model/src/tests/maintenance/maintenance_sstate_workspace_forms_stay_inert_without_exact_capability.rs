use super::*;

#[test]
fn maintenance_sstate_workspace_forms_stay_inert_without_exact_capability() {
    let mut state = MaintenanceState::default();
    assert_eq!(
        update_maintenance(&mut state, MaintenanceAction::OpenReadinessForm),
        MaintenanceTransition::none()
    );
    assert_eq!(
        update_maintenance(&mut state, MaintenanceAction::OpenCleanupForm),
        MaintenanceTransition::none()
    );
}
