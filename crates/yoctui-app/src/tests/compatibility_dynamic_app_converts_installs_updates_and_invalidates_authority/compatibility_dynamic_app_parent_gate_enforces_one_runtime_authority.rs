use super::*;

#[test]
fn compatibility_dynamic_app_parent_gate_enforces_one_runtime_authority() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.workspace.build_dir = Some("/work/poky/build".into());
    yoctui_model::install_workspace_compatibility(&mut app, compatibility_workspace_authority(9))
        .unwrap();

    let command = app
        .command_palette_commands()
        .into_iter()
        .find(|command| command.id == yoctui_model::CommandId::BuildImage)
        .unwrap();
    let workspace = yoctui_model::compatibility_ui_workspace_action_presentations(
        &app.workspace_compatibility,
        yoctui_model::WorkspaceDestination::Dashboard,
    )
    .into_iter()
    .find(|action| action.id == "dashboard.build")
    .unwrap();
    app.dialogs.push_front(yoctui_model::Dialog::BuildOptions);
    let dialog = yoctui_model::compatibility_ui_dialog_action_availability(
        &app.workspace_compatibility,
        app.active_dialog().unwrap(),
    );
    assert!(command.enabled());
    assert!(workspace.availability.enabled);
    assert!(dialog.enabled);
    assert_eq!(
        command.implementations,
        workspace.availability.implementations
    );
    assert_eq!(command.implementations, dialog.implementations);

    yoctui_model::invalidate_workspace_compatibility(&mut app);
    assert!(app.active_dialog().is_none());
    app.command_palette_open = true;
    app.command_palette_query = "Build image".into();
    app.command_palette_selection = 0;
    app.focus = yoctui_model::FocusTarget::CommandPalette;
    assert_eq!(
        compatibility_workspace_action(&mut app, yoctui_model::Action::ActivateCommandPalette),
        None
    );
    assert!(app.active_dialog().is_none());
    assert!(app.command_palette_open);

    app.dialogs
        .push_front(yoctui_model::Dialog::QuitConfirmation);
    let local = yoctui_model::compatibility_ui_dialog_action_availability(
        &app.workspace_compatibility,
        app.active_dialog().unwrap(),
    );
    assert!(local.enabled);
}
