use super::*;

#[test]
fn compatibility_workspace_app_routes_local_effects_and_denies_unavailable_effects() {
    let mut app = yoctui_model::App::new(16, 4096);
    assert_eq!(
        compatibility_workspace_action(
            &mut app,
            yoctui_model::Action::ChangeSelectedSetting { backwards: false },
        ),
        Some(yoctui_model::Effect::PersistSettings)
    );

    let before = app.package_inventory.clone();
    assert_eq!(
        compatibility_workspace_action(&mut app, yoctui_model::Action::BeginPackageInventory,),
        None
    );
    assert_eq!(app.package_inventory, before);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("No current environment capability snapshot")
    );
}
