use super::*;

#[test]
fn maintenance_workflow_preserves_selection_across_fixed_views() {
    let mut state = MaintenanceState::default();
    update_maintenance(
        &mut state,
        MaintenanceAction::Select {
            delta: 3,
            row_count: 8,
        },
    );
    update_maintenance(
        &mut state,
        MaintenanceAction::CycleView { backwards: false },
    );
    update_maintenance(
        &mut state,
        MaintenanceAction::Select {
            delta: 1,
            row_count: 3,
        },
    );
    update_maintenance(&mut state, MaintenanceAction::CycleView { backwards: true });
    assert_eq!(state.view, MaintenanceView::Sstate);
    assert_eq!(state.selection(), 3);
    assert_eq!(state.selections[MaintenanceView::Services.index()], 1);
}
