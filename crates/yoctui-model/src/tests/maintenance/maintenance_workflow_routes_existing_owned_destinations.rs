use super::*;

#[test]
fn maintenance_workflow_routes_existing_owned_destinations() {
    let mut state = MaintenanceState::default();
    for (action, screen) in [
        (MaintenanceAction::OpenSignatures, Screen::Signatures),
        (MaintenanceAction::OpenSecurity, Screen::Security),
        (MaintenanceAction::OpenQa, Screen::Qa),
        (MaintenanceAction::OpenRecipes, Screen::Recipes),
    ] {
        assert_eq!(
            update_maintenance(&mut state, action).effect,
            Some(MaintenanceEffect::Navigate(screen))
        );
    }
}
