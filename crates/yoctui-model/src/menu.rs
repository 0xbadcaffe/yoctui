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
    View,
    Tools,
    Help,
}

impl ApplicationMenuGroup {
    pub const ALL: [Self; 7] = [
        Self::Workspace,
        Self::Build,
        Self::Actions,
        Self::Navigate,
        Self::View,
        Self::Tools,
        Self::Help,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Workspace => "Workspace",
            Self::Build => "Build",
            Self::Actions => "Actions",
            Self::Navigate => "Navigate",
            Self::View => "View",
            Self::Tools => "Tools",
            Self::Help => "Help",
        }
    }

    pub const fn for_command(command: CommandId) -> Self {
        match command {
            CommandId::EditBbmask => Self::Workspace,
            CommandId::BuildImage | CommandId::BuildSelectedRecipe => Self::Build,
            CommandId::SelectImage
            | CommandId::OpenDashboard
            | CommandId::OpenLayers
            | CommandId::OpenRecipes
            | CommandId::OpenPackages
            | CommandId::OpenImages
            | CommandId::OpenSdk
            | CommandId::OpenDependencies
            | CommandId::OpenTesting
            | CommandId::OpenSecurity
            | CommandId::OpenQa
            | CommandId::OpenTasks
            | CommandId::OpenLogs
            | CommandId::OpenErrors
            | CommandId::OpenConfiguration
            | CommandId::ScrollFirst
            | CommandId::ScrollLast => Self::Navigate,
            CommandId::ChooseTheme
            | CommandId::FocusNavigator
            | CommandId::FocusWorkspace
            | CommandId::FocusInspector
            | CommandId::PreviousSubfocus
            | CommandId::NextSubfocus
            | CommandId::TogglePaneZoom => Self::View,
            CommandId::OpenGitUi
            | CommandId::OpenRawMode
            | CommandId::OpenTerminalSessions
            | CommandId::OpenMaintenance
            | CommandId::OpenBuildEnvironment
            | CommandId::OpenCompatibility
            | CommandId::OpenSettings => Self::Tools,
            CommandId::OpenOnboarding | CommandId::OpenHelp | CommandId::OpenAbout => Self::Help,
        }
    }
}

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
