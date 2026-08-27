use crate::{
    CommandId, OperatorActionId, OperatorActionSafety, OperatorActionTarget, WorkspaceDestination,
};

pub const MAX_MENU_PREFIX_CHARS: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationMenuGroup {
    Workspace,
    Build,
    Navigate,
    View,
    Tools,
    Help,
}

impl ApplicationMenuGroup {
    pub const ALL: [Self; 6] = [
        Self::Workspace,
        Self::Build,
        Self::Navigate,
        Self::View,
        Self::Tools,
        Self::Help,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Workspace => "Workspace",
            Self::Build => "Build",
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
            | CommandId::OpenImages
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
            CommandId::OpenRawMode
            | CommandId::OpenTerminalSessions
            | CommandId::OpenCompatibility
            | CommandId::OpenSettings => Self::Tools,
            CommandId::OpenHelp => Self::Help,
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
mod tests {
    use super::*;
    use crate::{Action, App, FocusTarget, Screen, update};
    use std::path::PathBuf;

    #[test]
    fn ux_menu_groups_context_availability_prefix_and_focus_are_typed_and_bounded() {
        let mut app = App::new(10, 1_000);
        let _ = update(&mut app, Action::OpenApplicationMenu);
        assert!(app.menu.is_open());
        assert_eq!(app.focus, FocusTarget::Dialog);
        assert_eq!(
            ApplicationMenuGroup::ALL.map(ApplicationMenuGroup::label),
            ["Workspace", "Build", "Navigate", "View", "Tools", "Help"]
        );
        assert_eq!(app.active_menu_items()[0].label, "Edit BBMASK");
        assert_eq!(
            app.active_menu_items()[0].disabled_reason.as_deref(),
            Some("Load a Yocto workspace first")
        );

        let _ = update(&mut app, Action::SelectMenuGroup { delta: 2 });
        for character in "open layers".chars() {
            let _ = update(&mut app, Action::AppendMenuPrefix(character));
        }
        assert_eq!(app.selected_menu_item().unwrap().label, "Open Layers");
        for _ in 0..64 {
            let _ = update(&mut app, Action::AppendMenuPrefix('x'));
        }
        assert_eq!(app.menu.typed_prefix.chars().count(), MAX_MENU_PREFIX_CHARS);
        let _ = update(&mut app, Action::SelectMenuItem { delta: 999 });
        assert_eq!(
            app.menu.item_selection,
            app.active_menu_items().len().saturating_sub(1)
        );

        let _ = update(&mut app, Action::CloseMenu);
        assert_eq!(app.focus, FocusTarget::Workspace);
        app.screen = Screen::Recipes;
        app.workspace.build_dir = Some(PathBuf::from("/work/build"));
        let _ = update(&mut app, Action::OpenContextMenu);
        let build = app
            .active_menu_items()
            .into_iter()
            .find(|item| item.action_id.as_str() == "recipes.build")
            .unwrap();
        assert_eq!(
            build.disabled_reason.as_deref(),
            Some("Select a recipe first.")
        );
        assert_eq!(build.safety, OperatorActionSafety::ConfirmationRequired);
    }
}
