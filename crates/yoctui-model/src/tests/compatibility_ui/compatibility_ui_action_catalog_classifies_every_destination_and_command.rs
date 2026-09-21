use super::*;

#[test]
fn compatibility_ui_action_catalog_classifies_every_destination_and_command() {
    let screens = [
        Screen::Dashboard,
        Screen::Tasks,
        Screen::BuildHistory,
        Screen::Dependencies,
        Screen::Signatures,
        Screen::LayerRelationships,
        Screen::Recipes,
        Screen::Packages,
        Screen::Images,
        Screen::Sdk,
        Screen::Testing,
        Screen::Security,
        Screen::Qa,
        Screen::RawMode,
        Screen::Layers,
        Screen::Configuration,
        Screen::Bbmask,
        Screen::Maintenance,
        Screen::Logs,
        Screen::Errors,
        Screen::Help,
        Screen::BuildEnvironment,
        Screen::Compatibility,
        Screen::Settings,
    ];
    for screen in screens {
        assert_eq!(
            compatibility_ui_destination_action_definition(screen).activation,
            CompatibilityUiActionActivation::Inspectable
        );
    }

    let commands = [
        CommandId::BuildImage,
        CommandId::SelectImage,
        CommandId::BuildSelectedRecipe,
        CommandId::EditBbmask,
        CommandId::OpenDashboard,
        CommandId::OpenLayers,
        CommandId::OpenRecipes,
        CommandId::OpenImages,
        CommandId::OpenTasks,
        CommandId::OpenLogs,
        CommandId::OpenErrors,
        CommandId::OpenConfiguration,
        CommandId::OpenRawMode,
        CommandId::OpenCompatibility,
        CommandId::OpenSettings,
        CommandId::ChooseTheme,
        CommandId::OpenHelp,
    ];
    for command in commands {
        let _ = compatibility_ui_command_action_definition(command);
    }
    assert_eq!(
        compatibility_ui_command_action_definition(CommandId::BuildImage).activation,
        CompatibilityUiActionActivation::CapabilityGated
    );
    assert_eq!(
        compatibility_ui_command_action_definition(CommandId::OpenLayers).activation,
        CompatibilityUiActionActivation::Inspectable
    );
    assert_eq!(
        compatibility_ui_command_action_definition(CommandId::ChooseTheme).activation,
        CompatibilityUiActionActivation::ClientLocal
    );
}
