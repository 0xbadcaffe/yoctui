use super::*;

#[test]
fn ux_action_catalog_drives_palette_metadata_search_and_workspace_projection() {
    yoctui_model::validate_operator_action_catalog().unwrap();
    let mut app = yoctui_model::App::new(16, 4096);
    app.command_palette_query = "capabilities".into();
    let commands = app.filtered_command_palette_commands();
    assert_eq!(commands.len(), 1);
    let command = &commands[0];
    assert_eq!(command.action_id.as_str(), "navigate.compatibility");
    assert_eq!(command.id, yoctui_model::CommandId::OpenCompatibility);
    assert_eq!(command.menu_path, ["Navigate", "Open Compatibility"]);
    assert!(command.aliases.contains(&"capabilities"));
    assert_eq!(
        command.help_group,
        yoctui_model::OperatorActionHelpGroup::Navigate
    );

    let build = app
        .command_palette_commands()
        .into_iter()
        .find(|command| command.id == yoctui_model::CommandId::BuildImage)
        .unwrap();
    assert_eq!(build.shortcut, "B");
    assert_eq!(key_action(Input::Char('B')), Some(Action::OpenBuildOptions));

    let catalog = yoctui_model::workspace_operator_action_definitions(
        yoctui_model::WorkspaceDestination::Images,
    );
    let presentations = yoctui_model::compatibility_ui_workspace_action_presentations(
        &app.workspace_compatibility,
        yoctui_model::WorkspaceDestination::Images,
    );
    assert_eq!(catalog.len(), presentations.len());
    for (definition, presentation) in catalog.iter().zip(&presentations) {
        assert_eq!(definition.id.as_str(), presentation.id);
        assert_eq!(definition.label, presentation.label);
        assert_eq!(definition.description, presentation.description);
        assert_eq!(definition.menu_path, presentation.menu_path);
        assert_eq!(definition.safety, presentation.safety);
        assert_eq!(definition.footer_priority, presentation.footer_priority);
        assert_eq!(definition.help_group, presentation.help_group);
    }
}
