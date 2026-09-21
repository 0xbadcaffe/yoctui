use super::*;

#[test]
fn maintenance_workflow_maps_first_class_screen_and_typed_keys() {
    let mut app = App::new(10, 1_000);
    assert_eq!(
        update(&mut app, Action::Open(Screen::Maintenance)),
        Some(yoctui_model::Effect::Maintenance(
            yoctui_model::MaintenanceEffect::InspectCapability { request: 1 }
        ))
    );
    assert_eq!(app.screen, Screen::Maintenance);
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Sstate, 4, Input::Char(']')),
        Some(Action::Maintenance(MaintenanceAction::CycleView {
            backwards: false
        }))
    );
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Sstate, 4, Input::Down),
        Some(Action::Maintenance(MaintenanceAction::Select {
            delta: 1,
            row_count: 4
        }))
    );
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Sstate, 4, Input::Char('S')),
        Some(Action::Maintenance(MaintenanceAction::OpenSignatures))
    );
}
