//! Navigation types.
use super::*;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AppError {
    #[error("{category}: {message}. {remedy}")]
    Message {
        category: &'static str,
        message: String,
        remedy: String,
    },
}
impl AppError {
    pub fn new(
        category: &'static str,
        message: impl Into<String>,
        remedy: impl Into<String>,
    ) -> Self {
        Self::Message {
            category,
            message: message.into(),
            remedy: remedy.into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Screen {
    Dashboard,
    Insights,
    Tasks,
    BuildHistory,
    Dependencies,
    Signatures,
    LayerRelationships,
    Recipes,
    Packages,
    Images,
    Kernel,
    Firmware,
    Sdk,
    Testing,
    Security,
    Qa,
    Layers,
    Configuration,
    Bbmask,
    RawMode,
    TerminalSessions,
    Maintenance,
    Logs,
    Errors,
    Help,
    BuildEnvironment,
    Compatibility,
    Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FunctionKey {
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FunctionShortcutRoute {
    Open(Screen),
    CommandPalette,
    ApplicationMenu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FunctionShortcut {
    pub key: FunctionKey,
    pub key_label: &'static str,
    pub action_label: &'static str,
    pub route: FunctionShortcutRoute,
}

pub const FUNCTION_SHORTCUTS: [FunctionShortcut; 10] = [
    FunctionShortcut {
        key: FunctionKey::F1,
        key_label: "F1",
        action_label: "Help",
        route: FunctionShortcutRoute::Open(Screen::Help),
    },
    FunctionShortcut {
        key: FunctionKey::F2,
        key_label: "F2",
        action_label: "Tasks",
        route: FunctionShortcutRoute::Open(Screen::Tasks),
    },
    FunctionShortcut {
        key: FunctionKey::F3,
        key_label: "F3",
        action_label: "History",
        route: FunctionShortcutRoute::Open(Screen::BuildHistory),
    },
    FunctionShortcut {
        key: FunctionKey::F4,
        key_label: "F4",
        action_label: "Dashboard",
        route: FunctionShortcutRoute::Open(Screen::Dashboard),
    },
    FunctionShortcut {
        key: FunctionKey::F5,
        key_label: "F5",
        action_label: "Logs",
        route: FunctionShortcutRoute::Open(Screen::Logs),
    },
    FunctionShortcut {
        key: FunctionKey::F6,
        key_label: "F6",
        action_label: "Layers",
        route: FunctionShortcutRoute::Open(Screen::Layers),
    },
    FunctionShortcut {
        key: FunctionKey::F7,
        key_label: "F7",
        action_label: "Recipes",
        route: FunctionShortcutRoute::Open(Screen::Recipes),
    },
    FunctionShortcut {
        key: FunctionKey::F8,
        key_label: "F8",
        action_label: "Images",
        route: FunctionShortcutRoute::Open(Screen::Images),
    },
    FunctionShortcut {
        key: FunctionKey::F9,
        key_label: "F9",
        action_label: "Commands",
        route: FunctionShortcutRoute::CommandPalette,
    },
    FunctionShortcut {
        key: FunctionKey::F10,
        key_label: "F10",
        action_label: "Menu",
        route: FunctionShortcutRoute::ApplicationMenu,
    },
];

pub fn function_shortcut_action(key: FunctionKey) -> Action {
    let shortcut = FUNCTION_SHORTCUTS
        .iter()
        .find(|shortcut| shortcut.key == key)
        .expect("the closed function-key catalog contains every FunctionKey");
    match shortcut.route {
        FunctionShortcutRoute::Open(screen) => Action::Open(screen),
        FunctionShortcutRoute::CommandPalette => Action::OpenCommandPalette,
        FunctionShortcutRoute::ApplicationMenu => Action::OpenApplicationMenu,
    }
}
/// The one active target in Yoctui's persistent workbench shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FocusTarget {
    Navigator,
    Workspace,
    Inspector,
    Dialog,
    CommandPalette,
}

impl FocusTarget {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Navigator => "Navigator",
            Self::Workspace => "Workspace",
            Self::Inspector => "Inspector",
            Self::Dialog => "Dialog",
            Self::CommandPalette => "Command Palette",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorMode {
    Navigator,
    DaemonSession,
    Task,
    Job,
    Dependency,
    Signature,
    Recipe,
    Package,
    Artifact,
    Test,
    Security,
    Qa,
    Layer,
    File,
    Configuration,
    Utility,
    RawCommand,
    Log,
    Error,
    Help,
    BuildEnvironment,
    CompatibilityCapability,
    Settings,
}

impl InspectorMode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Navigator => "Navigator",
            Self::DaemonSession => "Daemon / session",
            Self::Task => "Task",
            Self::Job => "Job",
            Self::Dependency => "Dependency",
            Self::Signature => "Signature",
            Self::Recipe => "Recipe",
            Self::Package => "Package",
            Self::Artifact => "Artifact",
            Self::Test => "Test",
            Self::Security => "Security",
            Self::Qa => "QA",
            Self::Layer => "Layer",
            Self::File => "File",
            Self::Configuration => "Configuration",
            Self::Utility => "Utility",
            Self::RawCommand => "Raw command",
            Self::Log => "Log",
            Self::Error => "Error",
            Self::Help => "Help",
            Self::BuildEnvironment => "Build environment",
            Self::CompatibilityCapability => "Compatibility capability",
            Self::Settings => "Settings",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum Theme {
    #[default]
    #[serde(alias = "dark")]
    DarkPro,
    #[serde(alias = "light")]
    WhiteClassic,
    MatrixGreen,
    VscodeDark,
    VscodeLight,
    AccessibleDark,
    SoftLight,
    HighContrast,
    /// Legacy persisted setting; new selectors expose the Packrat catalog only.
    Monochrome,
}

impl Theme {
    /// Stable, user-facing color names. Persisted enum names remain unchanged so
    /// existing preferences continue to deserialize across upgrades.
    pub const fn display_name(self) -> &'static str {
        match self {
            Self::DarkPro => "Dark blue",
            Self::WhiteClassic => "White",
            Self::MatrixGreen => "Green",
            Self::VscodeDark => "Dark gray",
            Self::VscodeLight => "Light gray",
            Self::AccessibleDark => "Accessible dark",
            Self::SoftLight => "Soft white",
            Self::HighContrast => "High contrast",
            Self::Monochrome => "Monochrome",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ClientAccessOrigin {
    #[default]
    Local,
    Ssh {
        client_ip: String,
    },
    SshUnknown,
}

impl ClientAccessOrigin {
    pub fn label(&self) -> String {
        match self {
            Self::Local => "Local".into(),
            Self::Ssh { client_ip } => format!("SSH {client_ip}"),
            Self::SshUnknown => "SSH (IP unavailable)".into(),
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum AnimationSpeed {
    Slow,
    #[default]
    Fast,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandId {
    BuildImage,
    SelectImage,
    BuildSelectedRecipe,
    EditBbmask,
    OpenDashboard,
    OpenLayers,
    OpenRecipes,
    OpenPackages,
    OpenImages,
    OpenSdk,
    OpenDependencies,
    OpenTesting,
    OpenSecurity,
    OpenQa,
    OpenTasks,
    OpenLogs,
    OpenErrors,
    OpenConfiguration,
    OpenRawMode,
    OpenTerminalSessions,
    OpenGitUi,
    OpenMaintenance,
    OpenBuildEnvironment,
    OpenCompatibility,
    OpenSettings,
    ChooseTheme,
    FocusNavigator,
    FocusWorkspace,
    FocusInspector,
    PreviousSubfocus,
    NextSubfocus,
    TogglePaneZoom,
    ScrollFirst,
    ScrollLast,
    OpenOnboarding,
    OpenHelp,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CommandPaletteMode {
    #[default]
    Commands,
    GlobalRegexSearch,
}
pub const MAX_COMMAND_PALETTE_QUERY_CHARS: usize = 256;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteCommand {
    pub action_id: OperatorActionId,
    pub id: CommandId,
    pub label: &'static str,
    pub description: String,
    pub shortcut: &'static str,
    pub menu_path: Vec<&'static str>,
    pub aliases: Vec<&'static str>,
    pub palette_keywords: Vec<&'static str>,
    pub safety: OperatorActionSafety,
    pub footer_priority: u8,
    pub help_group: OperatorActionHelpGroup,
    pub disabled_reason: Option<String>,
    pub compatibility_state: WorkspaceAvailabilityState,
    pub compatibility_reason: Option<String>,
    pub compatibility_limitations: Vec<String>,
    pub implementations: Vec<(CapabilityId, String)>,
}
impl PaletteCommand {
    pub fn enabled(&self) -> bool {
        self.disabled_reason.is_none()
    }
}
pub(crate) const NAVIGATOR_SCREENS: [Screen; 25] = [
    Screen::Dashboard,
    Screen::Insights,
    Screen::Layers,
    Screen::Recipes,
    Screen::Packages,
    Screen::Images,
    Screen::Kernel,
    Screen::Firmware,
    Screen::Sdk,
    Screen::Tasks,
    Screen::Logs,
    Screen::Errors,
    Screen::Configuration,
    Screen::Dependencies,
    Screen::Testing,
    Screen::Security,
    Screen::Qa,
    Screen::RawMode,
    Screen::TerminalSessions,
    Screen::Recipes,
    Screen::Images,
    Screen::Maintenance,
    Screen::BuildEnvironment,
    Screen::Compatibility,
    Screen::Settings,
];
pub(crate) const NAVIGATOR_COMPATIBILITY_DESTINATIONS: [WorkspaceDestination; 25] = [
    WorkspaceDestination::Dashboard,
    WorkspaceDestination::Dashboard,
    WorkspaceDestination::Layers,
    WorkspaceDestination::Recipes,
    WorkspaceDestination::Packages,
    WorkspaceDestination::Images,
    WorkspaceDestination::Kernel,
    WorkspaceDestination::Firmware,
    WorkspaceDestination::Sdk,
    WorkspaceDestination::Tasks,
    WorkspaceDestination::Logs,
    WorkspaceDestination::Errors,
    WorkspaceDestination::Configuration,
    WorkspaceDestination::Dependencies,
    WorkspaceDestination::Testing,
    WorkspaceDestination::Security,
    WorkspaceDestination::Qa,
    WorkspaceDestination::RawMode,
    WorkspaceDestination::TerminalSessions,
    WorkspaceDestination::Devtool,
    WorkspaceDestination::QemuWic,
    WorkspaceDestination::Maintenance,
    WorkspaceDestination::BuildEnvironment,
    WorkspaceDestination::Compatibility,
    WorkspaceDestination::Settings,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavigatorGroupRange {
    pub label: &'static str,
    pub start: usize,
    pub end: usize,
}

pub const NAVIGATOR_GROUPS: [NavigatorGroupRange; 5] = [
    NavigatorGroupRange {
        label: "OVERVIEW",
        start: 0,
        end: 2,
    },
    NavigatorGroupRange {
        label: "CONTENT",
        start: 2,
        end: 9,
    },
    NavigatorGroupRange {
        label: "BUILD",
        start: 9,
        end: 14,
    },
    NavigatorGroupRange {
        label: "VALIDATE",
        start: 14,
        end: 17,
    },
    NavigatorGroupRange {
        label: "TOOLS",
        start: 17,
        end: 25,
    },
];

/// Guidance and failure notifications require an explicit acknowledgement in
/// the client instead of relying on the easy-to-miss transient status line.
pub fn notification_requires_acknowledgement(message: &str) -> bool {
    message.starts_with("Select ")
        || message.starts_with("No ")
        || message.contains(" unavailable")
        || message.contains(" cannot ")
        || message.contains(" could not ")
        || message.contains(" failed")
}

pub(crate) fn navigator_group_for_selection(selection: usize) -> usize {
    NAVIGATOR_GROUPS
        .iter()
        .position(|group| (group.start..group.end).contains(&selection))
        .unwrap_or(0)
}
