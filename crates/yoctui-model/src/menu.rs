use crate::{
    CommandId, OperatorActionId, OperatorActionSafety, OperatorActionTarget, WorkspaceDestination,
};

pub const MAX_MENU_PREFIX_CHARS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationMenuGroup {
    Workspace,
    Build,
    Actions,
    Navigate,
    Configuration,
    View,
    Devtool,
    Tools,
    Help,
}

impl ApplicationMenuGroup {
    pub const ALL: [Self; 9] = [
        Self::Workspace,
        Self::Build,
        Self::Actions,
        Self::Navigate,
        Self::Configuration,
        Self::View,
        Self::Devtool,
        Self::Tools,
        Self::Help,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Workspace => "Workspace",
            Self::Build => "Build",
            Self::Actions => "Actions",
            Self::Navigate => "Navigate",
            Self::Configuration => "Config",
            Self::View => "View",
            Self::Devtool => "Devtool",
            Self::Tools => "Tools",
            Self::Help => "Help",
        }
    }

    pub const fn for_command(command: CommandId) -> Self {
        match command {
            CommandId::OpenBuildEnvironment
            | CommandId::OpenCompatibility
            | CommandId::OpenTerminalSessions
            | CommandId::OpenCommandPalette
            | CommandId::Quit => Self::Workspace,
            CommandId::EditBbmask
            | CommandId::OpenBbmask
            | CommandId::OpenConfiguration
            | CommandId::OpenBitBakeConfigBuild
            | CommandId::OpenBitBakeLayersSaveBuildConf
            | CommandId::OpenBitBakeLayersCreateLayersSetup => Self::Configuration,
            CommandId::OpenInsights
            | CommandId::OpenBuildHistory
            | CommandId::OpenSignatures
            | CommandId::OpenLayerRelationships
            | CommandId::OpenKernel
            | CommandId::OpenFirmware => Self::Navigate,
            CommandId::SelectImage | CommandId::BuildImage | CommandId::BuildSelectedRecipe => {
                Self::Build
            }
            CommandId::OpenDashboard
            | CommandId::OpenLayers
            | CommandId::OpenRecipes
            | CommandId::OpenPackages
            | CommandId::OpenImages
            | CommandId::OpenHardware
            | CommandId::OpenSdk
            | CommandId::OpenDependencies
            | CommandId::OpenTesting
            | CommandId::OpenSecurity
            | CommandId::OpenQa
            | CommandId::OpenTasks
            | CommandId::OpenLogs
            | CommandId::OpenErrors
            | CommandId::ScrollFirst
            | CommandId::ScrollLast => Self::Navigate,
            CommandId::ChooseTheme
            | CommandId::FocusNavigator
            | CommandId::FocusWorkspace
            | CommandId::FocusInspector
            | CommandId::PreviousSubfocus
            | CommandId::NextSubfocus
            | CommandId::TogglePaneZoom
            | CommandId::ToggleInspector
            | CommandId::OpenSettings => Self::View,
            CommandId::OpenDevtoolWorkspace | CommandId::OpenDevtool(_) => Self::Devtool,
            CommandId::OpenGitUi
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
            | CommandId::OpenRawMode
            | CommandId::OpenMaintenance => Self::Tools,
            CommandId::OpenOnboarding | CommandId::OpenHelp | CommandId::OpenAbout => Self::Help,
        }
    }
}

pub(crate) fn menu_item_order(group: ApplicationMenuGroup, item: &MenuItem) -> usize {
    use CommandId::*;
    let OperatorActionTarget::Command(command) = item.target else {
        return 100;
    };
    if group == ApplicationMenuGroup::Navigate {
        if let Some(screen) = command.screen() {
            return MENU_SCREENS
                .iter()
                .position(|candidate| *candidate == screen)
                .unwrap_or(100);
        }
        return if command == ScrollFirst { 101 } else { 102 };
    }
    let order: &[CommandId] = match group {
        ApplicationMenuGroup::Workspace => &[
            OpenBuildEnvironment,
            OpenCompatibility,
            OpenTerminalSessions,
            OpenCommandPalette,
            Quit,
        ],
        ApplicationMenuGroup::Build => &[BuildImage, SelectImage, BuildSelectedRecipe],
        ApplicationMenuGroup::Configuration => &[
            OpenConfiguration,
            OpenBbmask,
            EditBbmask,
            OpenBitBakeConfigBuild,
            OpenBitBakeLayersSaveBuildConf,
            OpenBitBakeLayersCreateLayersSetup,
        ],
        ApplicationMenuGroup::View => &[
            OpenSettings,
            ChooseTheme,
            ToggleInspector,
            TogglePaneZoom,
            FocusNavigator,
            FocusWorkspace,
            FocusInspector,
            PreviousSubfocus,
            NextSubfocus,
        ],
        ApplicationMenuGroup::Devtool => &[
            OpenDevtoolWorkspace,
            OpenDevtool(crate::DevtoolUtilityCommand::Status),
            OpenDevtool(crate::DevtoolUtilityCommand::Search),
            OpenDevtool(crate::DevtoolUtilityCommand::LatestVersion),
            OpenDevtool(crate::DevtoolUtilityCommand::CheckUpgradeStatus),
            OpenDevtool(crate::DevtoolUtilityCommand::Add),
            OpenDevtool(crate::DevtoolUtilityCommand::Modify),
            OpenDevtool(crate::DevtoolUtilityCommand::Upgrade),
            OpenDevtool(crate::DevtoolUtilityCommand::FindRecipe),
            OpenDevtool(crate::DevtoolUtilityCommand::EditRecipe),
            OpenDevtool(crate::DevtoolUtilityCommand::Menuconfig),
            OpenDevtool(crate::DevtoolUtilityCommand::ConfigureHelp),
            OpenDevtool(crate::DevtoolUtilityCommand::UpdateRecipe),
            OpenDevtool(crate::DevtoolUtilityCommand::Rename),
            OpenDevtool(crate::DevtoolUtilityCommand::Build),
            OpenDevtool(crate::DevtoolUtilityCommand::BuildImage),
            OpenDevtool(crate::DevtoolUtilityCommand::IdeSdk),
            OpenDevtool(crate::DevtoolUtilityCommand::CreateWorkspace),
            OpenDevtool(crate::DevtoolUtilityCommand::Extract),
            OpenDevtool(crate::DevtoolUtilityCommand::Sync),
            OpenDevtool(crate::DevtoolUtilityCommand::Import),
            OpenDevtool(crate::DevtoolUtilityCommand::Export),
            OpenDevtool(crate::DevtoolUtilityCommand::DeployTarget),
            OpenDevtool(crate::DevtoolUtilityCommand::Finish),
            OpenDevtool(crate::DevtoolUtilityCommand::UndeployTarget),
            OpenDevtool(crate::DevtoolUtilityCommand::Reset),
        ],
        ApplicationMenuGroup::Tools => &[
            OpenGitUi,
            OpenMaintenance,
            OpenRawMode,
            OpenBitBakeLayersShowMachines,
            OpenBitBakeLayersShowLayers,
            OpenBitBakeLayersShowRecipes,
            OpenBitBakeLayersShowOverlayed,
            OpenBitBakeLayersShowAppends,
            OpenBitBakeLayersShowCrossDepends,
            OpenBitBakeLayersLayerIndexShowDepends,
            OpenBitBakeLayersCreateLayer,
            OpenBitBakeLayersAddLayer,
            OpenBitBakeLayersRemoveLayer,
            OpenBitBakeLayersLayerIndexFetch,
            OpenBitBakeLayersFlatten,
        ],
        ApplicationMenuGroup::Help => &[OpenHelp, OpenOnboarding, OpenAbout],
        ApplicationMenuGroup::Actions | ApplicationMenuGroup::Navigate => &[],
    };
    order
        .iter()
        .position(|candidate| *candidate == command)
        .unwrap_or(100)
}

pub(crate) const MENU_SCREENS: [crate::Screen; 30] = {
    use crate::Screen::*;
    [
        Dashboard,
        Insights,
        BuildHistory,
        Layers,
        LayerRelationships,
        Recipes,
        Devtool,
        Packages,
        Images,
        Hardware,
        Kernel,
        Firmware,
        Sdk,
        Tasks,
        Logs,
        Errors,
        Configuration,
        Bbmask,
        Dependencies,
        Signatures,
        Testing,
        Security,
        Qa,
        RawMode,
        TerminalSessions,
        Maintenance,
        BuildEnvironment,
        Compatibility,
        Settings,
        Help,
    ]
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuKind {
    Application,
    Context(WorkspaceDestination),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MenuItem {
    pub action_id: OperatorActionId,
    pub target: OperatorActionTarget,
    pub label: &'static str,
    pub description: String,
    pub shortcut: &'static str,
    pub disabled_reason: Option<String>,
    pub safety: OperatorActionSafety,
}

impl MenuItem {
    pub fn enabled(&self) -> bool {
        self.disabled_reason.is_none()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MenuState {
    pub kind: Option<MenuKind>,
    pub group_selection: usize,
    pub item_selection: usize,
    pub typed_prefix: String,
}

impl MenuState {
    pub fn is_open(&self) -> bool {
        self.kind.is_some()
    }

    pub fn open_application(&mut self) {
        self.kind = Some(MenuKind::Application);
        self.group_selection = 0;
        self.item_selection = 0;
        self.typed_prefix.clear();
    }

    pub fn open_context(&mut self, destination: WorkspaceDestination) {
        self.kind = Some(MenuKind::Context(destination));
        self.group_selection = 0;
        self.item_selection = 0;
        self.typed_prefix.clear();
    }

    pub fn close(&mut self) {
        self.kind = None;
        self.item_selection = 0;
        self.typed_prefix.clear();
    }

    pub fn group(&self) -> ApplicationMenuGroup {
        ApplicationMenuGroup::ALL[self
            .group_selection
            .min(ApplicationMenuGroup::ALL.len().saturating_sub(1))]
    }
}

#[cfg(test)]
#[path = "tests/menu/mod.rs"]
mod tests;
