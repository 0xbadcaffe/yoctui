use super::*;

#[test]
fn compatibility_workspace_app_cli_routes_local_effect_and_blocks_environment_spawn() {
    let mut app = App::new(16, 4096);
    assert_eq!(
        compatibility_workspace_action(
            &mut app,
            Action::ChangeSelectedSetting { backwards: false },
        ),
        Some(Effect::PersistSettings)
    );

    let inventory_before = app.package_inventory.clone();
    let effect = compatibility_workspace_action(&mut app, Action::BeginPackageInventory);
    assert!(
        effect.is_none(),
        "an unavailable action must not be routed to a process or job"
    );
    assert_eq!(app.package_inventory, inventory_before);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("No current environment capability snapshot")
    );
}
