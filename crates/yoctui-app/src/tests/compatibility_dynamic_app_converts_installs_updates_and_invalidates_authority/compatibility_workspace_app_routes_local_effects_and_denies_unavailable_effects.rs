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

#[test]
fn platform_inspection_routes_while_the_daemon_snapshot_is_absent() {
    let mut app = yoctui_model::App::new(16, 4096);

    assert_eq!(
        compatibility_workspace_action(&mut app, yoctui_model::Action::InspectKernel),
        Some(yoctui_model::Effect::InspectKernel)
    );
    assert_eq!(
        app.kernel.inventory,
        yoctui_model::PlatformInventoryState::Loading
    );
    assert_eq!(
        compatibility_workspace_action(&mut app, yoctui_model::Action::InspectFirmware),
        Some(yoctui_model::Effect::InspectFirmware)
    );
    assert_eq!(
        app.firmware.inventory,
        yoctui_model::PlatformInventoryState::Loading
    );
    assert_eq!(app.notification, None);
}
