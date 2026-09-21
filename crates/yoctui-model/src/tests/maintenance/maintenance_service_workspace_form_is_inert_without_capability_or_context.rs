use super::*;

#[test]
fn maintenance_service_workspace_form_is_inert_without_capability_or_context() {
    let mut state = MaintenanceState {
        view: MaintenanceView::Services,
        ..MaintenanceState::default()
    };
    assert_eq!(
        update_maintenance(
            &mut state,
            MaintenanceAction::OpenPrServiceForm(PrServiceOperation::Export),
        ),
        MaintenanceTransition::none()
    );
}
