pub fn global_operator_action_definitions() -> Vec<OperatorActionDefinition> {
    GLOBAL_COMMANDS
        .into_iter()
        .map(global_operator_action_definition)
        .collect()
}

pub fn global_operator_action_definition(command: CommandId) -> OperatorActionDefinition {
    let metadata = global_metadata(command);
    OperatorActionDefinition {
        id: OperatorActionId::new(metadata.id),
        scope: metadata.scope,
        menu_path: metadata.menu_path,
        label: metadata.label,
        description: metadata.description.into(),
        aliases: metadata.aliases.to_vec(),
        palette_keywords: metadata.keywords.to_vec(),
        shortcut: global_shortcut_label(command),
        default_bindings: metadata.bindings.to_vec(),
        requirement: compatibility_ui_command_action_definition(command).requirement,
        local_requirement: metadata.local_requirement,
        safety: metadata.safety,
        footer_priority: metadata.footer_priority,
        help_group: metadata.help_group,
        target: OperatorActionTarget::Command(command),
    }
}

const fn global_shortcut_label(command: CommandId) -> &'static str {
    match command {
        CommandId::BuildImage => "B",
        CommandId::SelectImage => "i",
        CommandId::BuildSelectedRecipe => "b",
        CommandId::EditBbmask => "x then e",
        CommandId::OpenDashboard => "Esc / F4",
        CommandId::OpenLayers => "y / F6",
        CommandId::OpenRecipes => "r / F7",
        CommandId::OpenPackages => "none",
        CommandId::OpenImages => "i / F8",
        CommandId::OpenSdk
        | CommandId::OpenDependencies
        | CommandId::OpenTesting
        | CommandId::OpenSecurity
        | CommandId::OpenQa => "none",
        CommandId::OpenTasks => "t / F2",
        CommandId::OpenLogs => "l / F5",
        CommandId::OpenErrors => "e",
        CommandId::OpenConfiguration => "v",
        CommandId::OpenRawMode => "Ctrl+P raw",
        CommandId::OpenGitUi => "F12 Tools",
        CommandId::OpenBitBakeConfigBuild
        | CommandId::OpenBitBakeLayersShowLayers
        | CommandId::OpenBitBakeLayersShowRecipes
        | CommandId::OpenBitBakeLayersShowOverlayed
        | CommandId::OpenBitBakeLayersShowAppends
        | CommandId::OpenBitBakeLayersShowCrossDepends
        | CommandId::OpenBitBakeLayersAddLayer
        | CommandId::OpenBitBakeLayersRemoveLayer
        | CommandId::OpenBitBakeLayersFlatten
        | CommandId::OpenBitBakeLayersLayerIndexFetch
        | CommandId::OpenBitBakeLayersLayerIndexShowDepends
        | CommandId::OpenBitBakeLayersCreateLayer
        | CommandId::OpenBitBakeLayersShowMachines
        | CommandId::OpenBitBakeLayersSaveBuildConf
        | CommandId::OpenBitBakeLayersCreateLayersSetup => "F12 Tools",
        CommandId::OpenTerminalSessions => "Ctrl+B t",
        CommandId::OpenMaintenance | CommandId::OpenBuildEnvironment => "none",
        CommandId::OpenCompatibility => "none",
        CommandId::OpenSettings => "none",
        CommandId::ChooseTheme => "Ctrl+P theme",
        CommandId::FocusNavigator
        | CommandId::FocusWorkspace
        | CommandId::FocusInspector
        | CommandId::PreviousSubfocus
        | CommandId::NextSubfocus
        | CommandId::TogglePaneZoom => "F12 View",
        CommandId::ScrollFirst => "gg / Home",
        CommandId::ScrollLast => "G / End",
        CommandId::OpenOnboarding => "F12 Help",
        CommandId::OpenHelp => "? / F1",
        CommandId::OpenAbout => "F12 Help",
    }
}

pub const fn command_destination(command: CommandId) -> Option<WorkspaceDestination> {
    match command {
        CommandId::OpenDashboard => Some(WorkspaceDestination::Dashboard),
        CommandId::OpenLayers => Some(WorkspaceDestination::Layers),
        CommandId::OpenRecipes => Some(WorkspaceDestination::Recipes),
        CommandId::OpenPackages => Some(WorkspaceDestination::Packages),
        CommandId::OpenImages => Some(WorkspaceDestination::Images),
        CommandId::OpenSdk => Some(WorkspaceDestination::Sdk),
        CommandId::OpenDependencies => Some(WorkspaceDestination::Dependencies),
        CommandId::OpenTesting => Some(WorkspaceDestination::Testing),
        CommandId::OpenSecurity => Some(WorkspaceDestination::Security),
        CommandId::OpenQa => Some(WorkspaceDestination::Qa),
        CommandId::OpenTasks => Some(WorkspaceDestination::Tasks),
        CommandId::OpenLogs => Some(WorkspaceDestination::Logs),
        CommandId::OpenErrors => Some(WorkspaceDestination::Errors),
        CommandId::OpenConfiguration => Some(WorkspaceDestination::Configuration),
        CommandId::OpenRawMode => Some(WorkspaceDestination::RawMode),
        CommandId::OpenGitUi => None,
        CommandId::OpenBitBakeConfigBuild
        | CommandId::OpenBitBakeLayersShowLayers
        | CommandId::OpenBitBakeLayersShowRecipes
        | CommandId::OpenBitBakeLayersShowOverlayed
        | CommandId::OpenBitBakeLayersShowAppends
        | CommandId::OpenBitBakeLayersShowCrossDepends
        | CommandId::OpenBitBakeLayersAddLayer
        | CommandId::OpenBitBakeLayersRemoveLayer
        | CommandId::OpenBitBakeLayersFlatten
        | CommandId::OpenBitBakeLayersLayerIndexFetch
        | CommandId::OpenBitBakeLayersLayerIndexShowDepends
        | CommandId::OpenBitBakeLayersCreateLayer
        | CommandId::OpenBitBakeLayersShowMachines
        | CommandId::OpenBitBakeLayersSaveBuildConf
        | CommandId::OpenBitBakeLayersCreateLayersSetup => None,
        CommandId::OpenTerminalSessions => Some(WorkspaceDestination::TerminalSessions),
        CommandId::OpenMaintenance => Some(WorkspaceDestination::Maintenance),
        CommandId::OpenBuildEnvironment => Some(WorkspaceDestination::BuildEnvironment),
        CommandId::OpenCompatibility => Some(WorkspaceDestination::Compatibility),
        CommandId::OpenSettings => Some(WorkspaceDestination::Settings),
        CommandId::OpenHelp | CommandId::OpenAbout => Some(WorkspaceDestination::Help),
        CommandId::BuildImage
        | CommandId::SelectImage
        | CommandId::BuildSelectedRecipe
        | CommandId::EditBbmask
        | CommandId::ChooseTheme
        | CommandId::FocusNavigator
        | CommandId::FocusWorkspace
        | CommandId::FocusInspector
        | CommandId::PreviousSubfocus
        | CommandId::NextSubfocus
        | CommandId::TogglePaneZoom
        | CommandId::ScrollFirst
        | CommandId::ScrollLast
        | CommandId::OpenOnboarding => None,
    }
}

pub fn global_operator_action_for_destination(
    destination: WorkspaceDestination,
) -> Option<OperatorActionDefinition> {
    GLOBAL_COMMANDS.into_iter().find_map(|command| {
        (command_destination(command) == Some(destination))
            .then(|| global_operator_action_definition(command))
    })
}
