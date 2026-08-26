use std::collections::HashSet;

use crate::{
    CommandId, CompatibilityUiWorkspaceActionDefinition, WorkspaceDestination,
    WorkspaceEffectRequirement, compatibility_ui_command_action_definition,
    compatibility_ui_workspace_action_seeds,
};

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

const GLOBAL_COMMANDS: [CommandId; 25] = [
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
    CommandId::FocusNavigator,
    CommandId::FocusWorkspace,
    CommandId::FocusInspector,
    CommandId::PreviousSubfocus,
    CommandId::NextSubfocus,
    CommandId::TogglePaneZoom,
    CommandId::ScrollFirst,
    CommandId::ScrollLast,
    CommandId::OpenHelp,
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

fn global_metadata(command: CommandId) -> GlobalMetadata {
    use OperatorActionHelpGroup as Group;
    use OperatorActionLocalRequirement as Local;
    use OperatorActionSafety as Safety;
    use OperatorActionScope as Scope;
    match command {
        CommandId::BuildImage => GlobalMetadata {
            id: "build.image",
            scope: Scope::Workspace(WorkspaceDestination::Images),
            menu_path: vec!["Build", "Build image"],
            label: "Build image",
            description: "Open image build options for the active machine",
            aliases: &["image build", "bitbake image"],
            keywords: &["build", "image", "machine", "bitbake"],
            bindings: &["B"],
            local_requirement: Local::WorkspaceLoaded,
            safety: Safety::ConfirmationRequired,
            footer_priority: 90,
            help_group: Group::Build,
        },
        CommandId::SelectImage => GlobalMetadata {
            id: "navigate.select-image",
            scope: Scope::Workspace(WorkspaceDestination::Images),
            menu_path: vec!["Navigate", "Select image"],
            label: "Select image",
            description: "Choose an image recipe discovered in active layers",
            aliases: &["image picker", "choose image"],
            keywords: &["navigate", "image", "recipe", "select"],
            bindings: &["i"],
            local_requirement: Local::ImageRecipeAvailable,
            safety: Safety::ReadOnly,
            footer_priority: 70,
            help_group: Group::Navigate,
        },
        CommandId::BuildSelectedRecipe => GlobalMetadata {
            id: "build.selected-recipe",
            scope: Scope::Workspace(WorkspaceDestination::Recipes),
            menu_path: vec!["Build", "Build selected recipe"],
            label: "Build selected recipe",
            description: "Confirm and build the selected recipe",
            aliases: &["recipe build", "bitbake recipe"],
            keywords: &["build", "recipe", "selected", "bitbake"],
            bindings: &["b"],
            local_requirement: Local::SelectedRecipe,
            safety: Safety::ConfirmationRequired,
            footer_priority: 90,
            help_group: Group::Build,
        },
        CommandId::EditBbmask => GlobalMetadata {
            id: "configure.bbmask",
            scope: Scope::Workspace(WorkspaceDestination::Configuration),
            menu_path: vec!["Workspace", "Configuration", "Edit BBMASK"],
            label: "Edit BBMASK",
            description: "Preview and save the effective BBMASK value",
            aliases: &["mask recipes", "configuration mask"],
            keywords: &["configure", "bbmask", "recipe", "mask"],
            bindings: &["x e"],
            local_requirement: Local::WorkspaceLoaded,
            safety: Safety::ConfirmationRequired,
            footer_priority: 55,
            help_group: Group::Configure,
        },
        CommandId::OpenDashboard => navigation(
            "navigate.dashboard",
            "Open Dashboard",
            "Show build status, task progress, and recent output",
            &["home", "overview"],
            &["dashboard", "home", "status", "overview"],
            &["Esc", "F4"],
            75,
        ),
        CommandId::OpenLayers => navigation(
            "navigate.layers",
            "Open Layers",
            "Browse active layer metadata and files",
            &["layer browser"],
            &["layers", "metadata", "files"],
            &["y", "F6"],
            65,
        ),
        CommandId::OpenRecipes => navigation(
            "navigate.recipes",
            "Open Recipes",
            "Browse recipes and typed recipe actions",
            &["recipe browser"],
            &["recipes", "metadata", "build"],
            &["r", "F7"],
            70,
        ),
        CommandId::OpenImages => navigation(
            "navigate.images",
            "Open Images",
            "Browse image recipes and artifacts",
            &["image browser"],
            &["images", "artifacts", "deploy"],
            &["i", "F8"],
            70,
        ),
        CommandId::OpenTasks => navigation(
            "navigate.tasks",
            "Open Tasks",
            "Inspect active and completed BitBake tasks",
            &["task monitor"],
            &["tasks", "jobs", "build", "progress"],
            &["t", "F2"],
            75,
        ),
        CommandId::OpenLogs => navigation(
            "navigate.logs",
            "Open Logs",
            "Inspect retained structured build logs",
            &["build output"],
            &["logs", "output", "build", "diagnostics"],
            &["l", "F5"],
            70,
        ),
        CommandId::OpenErrors => navigation(
            "navigate.errors",
            "Open Errors",
            "Inspect retained warnings and errors",
            &["diagnostics", "failures"],
            &["errors", "warnings", "failures", "diagnostics"],
            &["e"],
            70,
        ),
        CommandId::OpenConfiguration => navigation(
            "navigate.configuration",
            "Open Configuration",
            "Inspect effective BitBake variables and provenance",
            &["variables", "bitbake environment"],
            &["configuration", "variables", "provenance", "environment"],
            &["v"],
            60,
        ),
        CommandId::OpenRawMode => navigation(
            "navigate.raw-mode",
            "Open Raw Mode",
            "Inspect the structured BitBake command catalog",
            &["raw commands", "command workbench"],
            &["raw", "commands", "bitbake", "advanced"],
            &[],
            35,
        ),
        CommandId::OpenCompatibility => navigation(
            "navigate.compatibility",
            "Open Compatibility",
            "Inspect connected environment identity and capability evidence",
            &["capabilities", "support"],
            &["compatibility", "capability", "environment", "support"],
            &[],
            40,
        ),
        CommandId::OpenSettings => navigation(
            "navigate.settings",
            "Open Settings",
            "Edit persistent visual and log preferences",
            &["preferences"],
            &["settings", "preferences", "theme", "logs"],
            &[],
            40,
        ),
        CommandId::ChooseTheme => GlobalMetadata {
            id: "view.choose-theme",
            scope: Scope::Global,
            menu_path: vec!["View", "Choose theme"],
            label: "Choose theme",
            description: "Preview and apply a named workbench palette",
            aliases: &["appearance", "colors"],
            keywords: &["theme", "appearance", "color", "palette"],
            bindings: &[],
            local_requirement: Local::None,
            safety: Safety::ReadOnly,
            footer_priority: 20,
            help_group: Group::General,
        },
        CommandId::FocusNavigator => focus_metadata(
            "view.focus-navigator",
            "Focus Navigator",
            "Move input focus directly to the Navigator pane",
        ),
        CommandId::FocusWorkspace => focus_metadata(
            "view.focus-workspace",
            "Focus Workspace",
            "Move input focus directly to the active Workspace pane",
        ),
        CommandId::FocusInspector => focus_metadata(
            "view.focus-inspector",
            "Focus Inspector",
            "Move input focus directly to the Inspector pane",
        ),
        CommandId::PreviousSubfocus => focus_metadata(
            "view.previous-subfocus",
            "Previous subfocus",
            "Move to the previous logical section in the focused pane",
        ),
        CommandId::NextSubfocus => focus_metadata(
            "view.next-subfocus",
            "Next subfocus",
            "Move to the next logical section in the focused pane",
        ),
        CommandId::TogglePaneZoom => focus_metadata(
            "view.toggle-pane-zoom",
            "Toggle pane zoom",
            "Zoom or restore the focused pane without changing its state",
        ),
        CommandId::ScrollFirst => navigation(
            "navigate.collection-first",
            "First row",
            "Move to the first retained row in the active collection",
            &["top", "start"],
            &["navigate", "scroll", "first", "top", "collection"],
            &["g g", "Home"],
            55,
        ),
        CommandId::ScrollLast => navigation(
            "navigate.collection-last",
            "Last row",
            "Move to the last retained row in the active collection",
            &["bottom", "end"],
            &["navigate", "scroll", "last", "bottom", "collection"],
            &["G", "End"],
            55,
        ),
        CommandId::OpenHelp => GlobalMetadata {
            id: "help.open",
            scope: Scope::Global,
            menu_path: vec!["Help", "Open Help"],
            label: "Open Help",
            description: "Show global and contextual actions and shortcuts",
            aliases: &["shortcuts", "keys"],
            keywords: &["help", "shortcuts", "actions", "keys"],
            bindings: &["?", "F1"],
            local_requirement: Local::None,
            safety: Safety::ReadOnly,
            footer_priority: 100,
            help_group: Group::General,
        },
    }
}

fn focus_metadata(
    id: &'static str,
    label: &'static str,
    description: &'static str,
) -> GlobalMetadata {
    GlobalMetadata {
        id,
        scope: OperatorActionScope::Global,
        menu_path: vec!["View", label],
        label,
        description,
        aliases: &["pane focus", "zoom", "subfocus"],
        keywords: &["view", "pane", "focus", "zoom", "subfocus"],
        bindings: &[],
        local_requirement: OperatorActionLocalRequirement::None,
        safety: OperatorActionSafety::ReadOnly,
        footer_priority: 25,
        help_group: OperatorActionHelpGroup::General,
    }
}

fn navigation(
    id: &'static str,
    label: &'static str,
    description: &'static str,
    aliases: &'static [&'static str],
    keywords: &'static [&'static str],
    bindings: &'static [&'static str],
    footer_priority: u8,
) -> GlobalMetadata {
    GlobalMetadata {
        id,
        scope: OperatorActionScope::Global,
        menu_path: vec!["Navigate", label],
        label,
        description,
        aliases,
        keywords,
        bindings,
        local_requirement: OperatorActionLocalRequirement::None,
        safety: OperatorActionSafety::ReadOnly,
        footer_priority,
        help_group: OperatorActionHelpGroup::Navigate,
    }
}

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
        CommandId::OpenImages => "i / F8",
        CommandId::OpenTasks => "t / F2",
        CommandId::OpenLogs => "l / F5",
        CommandId::OpenErrors => "e",
        CommandId::OpenConfiguration => "v",
        CommandId::OpenRawMode => "Ctrl+P raw",
        CommandId::OpenCompatibility => "none",
        CommandId::OpenSettings => "none",
        CommandId::ChooseTheme => "Ctrl+P theme",
        CommandId::FocusNavigator
        | CommandId::FocusWorkspace
        | CommandId::FocusInspector
        | CommandId::PreviousSubfocus
        | CommandId::NextSubfocus
        | CommandId::TogglePaneZoom => "F10 View",
        CommandId::ScrollFirst => "gg / Home",
        CommandId::ScrollLast => "G / End",
        CommandId::OpenHelp => "? / F1",
    }
}

pub const fn command_destination(command: CommandId) -> Option<WorkspaceDestination> {
    match command {
        CommandId::OpenDashboard => Some(WorkspaceDestination::Dashboard),
        CommandId::OpenLayers => Some(WorkspaceDestination::Layers),
        CommandId::OpenRecipes => Some(WorkspaceDestination::Recipes),
        CommandId::OpenImages => Some(WorkspaceDestination::Images),
        CommandId::OpenTasks => Some(WorkspaceDestination::Tasks),
        CommandId::OpenLogs => Some(WorkspaceDestination::Logs),
        CommandId::OpenErrors => Some(WorkspaceDestination::Errors),
        CommandId::OpenConfiguration => Some(WorkspaceDestination::Configuration),
        CommandId::OpenRawMode => Some(WorkspaceDestination::RawMode),
        CommandId::OpenCompatibility => Some(WorkspaceDestination::Compatibility),
        CommandId::OpenSettings => Some(WorkspaceDestination::Settings),
        CommandId::OpenHelp => Some(WorkspaceDestination::Help),
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
        | CommandId::ScrollLast => None,
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

pub fn workspace_operator_action_definitions(
    destination: WorkspaceDestination,
) -> Vec<OperatorActionDefinition> {
    compatibility_ui_workspace_action_seeds(destination)
        .into_iter()
        .enumerate()
        .map(|(index, seed)| workspace_definition(destination, seed, index))
        .collect()
}

fn workspace_definition(
    destination: WorkspaceDestination,
    seed: CompatibilityUiWorkspaceActionDefinition,
    index: usize,
) -> OperatorActionDefinition {
    let mut aliases = Vec::new();
    if seed.shortcut.starts_with('/') {
        aliases.push("/");
    }
    aliases.extend(seed.shortcut.split('/').filter(|value| !value.is_empty()));
    let mut keywords = seed.id.split(['.', '_']).collect::<Vec<_>>();
    keywords.extend(
        seed.label
            .split_ascii_whitespace()
            .map(|value| value.trim_matches(|character: char| !character.is_ascii_alphanumeric())),
    );
    keywords.retain(|value| !value.is_empty());
    keywords.sort_unstable();
    keywords.dedup();
    OperatorActionDefinition {
        id: OperatorActionId::new(seed.id),
        scope: OperatorActionScope::Workspace(destination),
        menu_path: vec![destination.label(), seed.label],
        label: seed.label,
        description: format!("{} in the {} workspace", seed.label, destination.label()),
        aliases: aliases.clone(),
        palette_keywords: keywords,
        shortcut: seed.shortcut,
        default_bindings: aliases,
        requirement: seed.requirement,
        local_requirement: OperatorActionLocalRequirement::None,
        safety: workspace_safety(seed.id),
        footer_priority: 80_u8.saturating_sub(u8::try_from(index).unwrap_or(u8::MAX)),
        help_group: workspace_help_group(seed.id),
        target: OperatorActionTarget::Workspace {
            destination,
            legacy_id: seed.id,
        },
    }
}

fn workspace_safety(id: &str) -> OperatorActionSafety {
    if [
        ".remove",
        ".reset",
        ".cleanup",
        ".write",
        ".undeploy",
        ".cancel",
    ]
    .iter()
    .any(|fragment| id.contains(fragment))
    {
        OperatorActionSafety::DestructiveConfirmation
    } else if [
        ".build",
        ".force_task",
        ".create",
        ".deploy",
        ".finish",
        ".update",
        ".cve",
        ".spdx",
        ".qemu",
        ".wic",
        "sdk.",
        "testing.",
        "qa.recipe",
        "maintenance.archive",
        "maintenance.locked",
    ]
    .iter()
    .any(|fragment| id.contains(fragment))
    {
        OperatorActionSafety::ConfirmationRequired
    } else {
        OperatorActionSafety::ReadOnly
    }
}

fn workspace_help_group(id: &str) -> OperatorActionHelpGroup {
    if id.contains("build") || id.contains("force_task") {
        OperatorActionHelpGroup::Build
    } else if id.contains("inspect") || id.contains("open") || id.contains("evidence") {
        OperatorActionHelpGroup::Inspect
    } else {
        OperatorActionHelpGroup::Operate
    }
}

pub fn operator_action_catalog() -> Vec<OperatorActionDefinition> {
    let mut catalog = global_operator_action_definitions();
    for destination in WorkspaceDestination::ALL {
        catalog.extend(workspace_operator_action_definitions(destination));
    }
    catalog
}

pub fn validate_operator_action_catalog() -> Result<(), Vec<String>> {
    let catalog = operator_action_catalog();
    let mut errors = Vec::new();
    let mut ids = HashSet::new();
    for action in &catalog {
        let id = action.id.as_str();
        if !ids.insert(id) {
            errors.push(format!("duplicate action ID: {id}"));
        }
        if id.is_empty()
            || id.starts_with('.')
            || id.ends_with('.')
            || !id.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"-_.".contains(&byte)
            })
        {
            errors.push(format!("invalid stable action ID: {id}"));
        }
        if action.label.trim().is_empty()
            || action.description.trim().is_empty()
            || action.menu_path.len() < 2
            || action.menu_path.iter().any(|part| part.trim().is_empty())
            || action.palette_keywords.is_empty()
        {
            errors.push(format!("incomplete action metadata: {id}"));
        }
        if action.footer_priority > 100 {
            errors.push(format!("invalid footer priority: {id}"));
        }
        if action.safety == OperatorActionSafety::ReadOnly
            && [
                ".remove",
                ".reset",
                ".cleanup",
                ".write",
                ".undeploy",
                ".cancel",
            ]
            .iter()
            .any(|fragment| id.contains(fragment))
        {
            errors.push(format!("unsafe action is classified read-only: {id}"));
        }
    }
    for command in GLOBAL_COMMANDS {
        let count = catalog
            .iter()
            .filter(|action| action.target == OperatorActionTarget::Command(command))
            .count();
        if count != 1 {
            errors.push(format!("command {command:?} has {count} catalog entries"));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

impl WorkspaceDestination {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Recipes => "Recipes",
            Self::Layers => "Layers",
            Self::Configuration => "Configuration",
            Self::Tasks => "Tasks",
            Self::BuildHistory => "Build History",
            Self::Logs => "Logs",
            Self::Errors => "Errors",
            Self::Dependencies => "Dependencies",
            Self::Signatures => "Signatures",
            Self::Packages => "Packages",
            Self::Images => "Images",
            Self::Sdk => "SDK",
            Self::Testing => "Testing",
            Self::Security => "Security",
            Self::Qa => "QA",
            Self::RawMode => "Raw Mode",
            Self::Devtool => "Devtool",
            Self::QemuWic => "QEMU / Wic",
            Self::Maintenance => "Maintenance",
            Self::ProjectProfiles => "Project Profiles",
            Self::TerminalSessions => "Terminal Sessions",
            Self::BuildEnvironment => "Build Environment",
            Self::Compatibility => "Compatibility",
            Self::Settings => "Settings",
            Self::Help => "Help",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ux_action_catalog_is_unique_complete_and_safe() {
        validate_operator_action_catalog().unwrap();
        let catalog = operator_action_catalog();
        assert_eq!(catalog.len(), 125, "25 global plus 100 workspace actions");
        assert!(
            catalog
                .iter()
                .all(|action| !action.palette_keywords.is_empty())
        );
        assert_eq!(
            catalog
                .iter()
                .find(|action| action.id.as_str() == "build.image")
                .unwrap()
                .safety,
            OperatorActionSafety::ConfirmationRequired
        );
        assert_eq!(
            catalog
                .iter()
                .find(|action| action.id.as_str() == "layers.remove")
                .unwrap()
                .safety,
            OperatorActionSafety::DestructiveConfirmation
        );
    }

    #[test]
    fn ux_action_catalog_covers_every_workspace_seed_without_drift() {
        for destination in WorkspaceDestination::ALL {
            let seeds = compatibility_ui_workspace_action_seeds(destination);
            let definitions = workspace_operator_action_definitions(destination);
            assert_eq!(seeds.len(), definitions.len(), "{destination:?}");
            for (seed, definition) in seeds.iter().zip(&definitions) {
                assert_eq!(seed.id, definition.id.as_str());
                assert_eq!(seed.label, definition.label);
                assert_eq!(seed.shortcut, definition.shortcut);
                assert!(!definition.default_bindings.is_empty());
                assert_eq!(seed.requirement, definition.requirement);
            }
        }
    }

    #[test]
    fn ux_action_catalog_global_metadata_is_searchable_and_menu_ready() {
        for command in GLOBAL_COMMANDS {
            let action = global_operator_action_definition(command);
            assert!(matches!(action.target, OperatorActionTarget::Command(id) if id == command));
            assert!(action.menu_path.len() >= 2);
            assert!(!action.description.is_empty());
            assert!(!action.palette_keywords.is_empty());
            assert!(action.footer_priority <= 100);
        }
    }
}
