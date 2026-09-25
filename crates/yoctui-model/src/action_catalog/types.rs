use crate::DevtoolUtilityCommand;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OperatorActionId(&'static str);

impl OperatorActionId {
    pub const fn new(value: &'static str) -> Self {
        Self(value)
    }

    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorActionScope {
    Global,
    Workspace(WorkspaceDestination),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorActionSafety {
    ReadOnly,
    ConfirmationRequired,
    DestructiveConfirmation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorActionHelpGroup {
    General,
    Navigate,
    Build,
    Configure,
    Inspect,
    Operate,
}

impl OperatorActionHelpGroup {
    pub const fn label(self) -> &'static str {
        match self {
            Self::General => "General",
            Self::Navigate => "Navigate",
            Self::Build => "Build",
            Self::Configure => "Configure",
            Self::Inspect => "Inspect",
            Self::Operate => "Operate",
        }
    }
}

impl OperatorActionSafety {
    pub const fn label(self) -> &'static str {
        match self {
            Self::ReadOnly => "read-only",
            Self::ConfirmationRequired => "confirmation required",
            Self::DestructiveConfirmation => "destructive confirmation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorActionLocalRequirement {
    None,
    WorkspaceLoaded,
    ImageRecipeAvailable,
    SelectedRecipe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorActionTarget {
    Command(CommandId),
    Workspace {
        destination: WorkspaceDestination,
        legacy_id: &'static str,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperatorActionDefinition {
    pub id: OperatorActionId,
    pub scope: OperatorActionScope,
    pub menu_path: Vec<&'static str>,
    pub label: &'static str,
    pub description: String,
    pub aliases: Vec<&'static str>,
    pub palette_keywords: Vec<&'static str>,
    pub shortcut: &'static str,
    pub default_bindings: Vec<&'static str>,
    pub requirement: WorkspaceEffectRequirement,
    pub local_requirement: OperatorActionLocalRequirement,
    pub safety: OperatorActionSafety,
    pub footer_priority: u8,
    pub help_group: OperatorActionHelpGroup,
    pub target: OperatorActionTarget,
}

const GLOBAL_COMMANDS: [CommandId; 77] = [
    CommandId::BuildImage,
    CommandId::SelectImage,
    CommandId::BuildSelectedRecipe,
    CommandId::EditBbmask,
    CommandId::OpenDashboard,
    CommandId::OpenLayers,
    CommandId::OpenRecipes,
    CommandId::OpenPackages,
    CommandId::OpenImages,
    CommandId::OpenSdk,
    CommandId::OpenDependencies,
    CommandId::OpenTesting,
    CommandId::OpenSecurity,
    CommandId::OpenQa,
    CommandId::OpenTasks,
    CommandId::OpenLogs,
    CommandId::OpenErrors,
    CommandId::OpenConfiguration,
    CommandId::OpenRawMode,
    CommandId::OpenTerminalSessions,
    CommandId::OpenGitUi,
    CommandId::OpenDevtool(DevtoolUtilityCommand::Add),
    CommandId::OpenDevtool(DevtoolUtilityCommand::Modify),
    CommandId::OpenDevtool(DevtoolUtilityCommand::Upgrade),
    CommandId::OpenDevtool(DevtoolUtilityCommand::Status),
    CommandId::OpenDevtool(DevtoolUtilityCommand::LatestVersion),
    CommandId::OpenDevtool(DevtoolUtilityCommand::CheckUpgradeStatus),
    CommandId::OpenDevtool(DevtoolUtilityCommand::Search),
    CommandId::OpenDevtool(DevtoolUtilityCommand::Build),
    CommandId::OpenDevtool(DevtoolUtilityCommand::IdeSdk),
    CommandId::OpenDevtool(DevtoolUtilityCommand::Rename),
    CommandId::OpenDevtool(DevtoolUtilityCommand::EditRecipe),
    CommandId::OpenDevtool(DevtoolUtilityCommand::FindRecipe),
    CommandId::OpenDevtool(DevtoolUtilityCommand::ConfigureHelp),
    CommandId::OpenDevtool(DevtoolUtilityCommand::UpdateRecipe),
    CommandId::OpenDevtool(DevtoolUtilityCommand::Reset),
    CommandId::OpenDevtool(DevtoolUtilityCommand::Finish),
    CommandId::OpenDevtool(DevtoolUtilityCommand::DeployTarget),
    CommandId::OpenDevtool(DevtoolUtilityCommand::UndeployTarget),
    CommandId::OpenDevtool(DevtoolUtilityCommand::BuildImage),
    CommandId::OpenDevtool(DevtoolUtilityCommand::CreateWorkspace),
    CommandId::OpenDevtool(DevtoolUtilityCommand::Export),
    CommandId::OpenDevtool(DevtoolUtilityCommand::Extract),
    CommandId::OpenDevtool(DevtoolUtilityCommand::Sync),
    CommandId::OpenDevtool(DevtoolUtilityCommand::Import),
    CommandId::OpenDevtool(DevtoolUtilityCommand::Menuconfig),
    CommandId::OpenBitBakeConfigBuild,
    CommandId::OpenBitBakeLayersShowLayers,
    CommandId::OpenBitBakeLayersShowRecipes,
    CommandId::OpenBitBakeLayersShowOverlayed,
    CommandId::OpenBitBakeLayersShowAppends,
    CommandId::OpenBitBakeLayersShowCrossDepends,
    CommandId::OpenBitBakeLayersAddLayer,
    CommandId::OpenBitBakeLayersRemoveLayer,
    CommandId::OpenBitBakeLayersFlatten,
    CommandId::OpenBitBakeLayersLayerIndexFetch,
    CommandId::OpenBitBakeLayersLayerIndexShowDepends,
    CommandId::OpenBitBakeLayersCreateLayer,
    CommandId::OpenBitBakeLayersShowMachines,
    CommandId::OpenBitBakeLayersSaveBuildConf,
    CommandId::OpenBitBakeLayersCreateLayersSetup,
    CommandId::OpenMaintenance,
    CommandId::OpenBuildEnvironment,
    CommandId::OpenCompatibility,
    CommandId::OpenSettings,
    CommandId::ChooseTheme,
    CommandId::FocusNavigator,
    CommandId::FocusWorkspace,
    CommandId::FocusInspector,
    CommandId::PreviousSubfocus,
    CommandId::NextSubfocus,
    CommandId::TogglePaneZoom,
    CommandId::ScrollFirst,
    CommandId::ScrollLast,
    CommandId::OpenOnboarding,
    CommandId::OpenHelp,
    CommandId::OpenAbout,
];

struct GlobalMetadata {
    id: &'static str,
    scope: OperatorActionScope,
    menu_path: Vec<&'static str>,
    label: &'static str,
    description: &'static str,
    aliases: &'static [&'static str],
    keywords: &'static [&'static str],
    bindings: &'static [&'static str],
    local_requirement: OperatorActionLocalRequirement,
    safety: OperatorActionSafety,
    footer_priority: u8,
    help_group: OperatorActionHelpGroup,
}
