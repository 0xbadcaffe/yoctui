use super::*;

#[test]
fn maintenance_workflow_rejects_stale_capability_and_preview() {
    let mut state = MaintenanceState::default();
    update_maintenance(&mut state, MaintenanceAction::InspectCapability);
    update_maintenance(&mut state, MaintenanceAction::InspectCapability);
    update_maintenance(
        &mut state,
        MaintenanceAction::CapabilityLoaded {
            request: 1,
            snapshot: capability(),
            partial: false,
        },
    );
    assert!(matches!(
        state.capability,
        MaintenanceCapability::Loading(2)
    ));

    update_maintenance(
        &mut state,
        MaintenanceAction::CapabilityLoaded {
            request: 2,
            snapshot: capability(),
            partial: false,
        },
    );
    let stale = readiness_preview(3);
    assert_eq!(
        update_maintenance(&mut state, MaintenanceAction::BeginOperation(stale)),
        MaintenanceTransition::none()
    );
}
