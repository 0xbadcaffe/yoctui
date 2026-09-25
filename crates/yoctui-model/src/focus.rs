use crate::{FocusTarget, Screen};

pub fn focus_target_is_relevant(app: &crate::App, target: FocusTarget) -> bool {
    match target {
        FocusTarget::Navigator => true,
        FocusTarget::Workspace => match app.screen {
            // These are informational projections. Their content never
            // becomes actionable merely because live or retained data appears.
            Screen::Dashboard | Screen::LayerRelationships | Screen::Help => false,
            _ => true,
        },
        // Inspector surfaces currently project facts and context actions but
        // own no selectable, scrollable, editable, or terminal-input control.
        FocusTarget::Inspector => false,
        FocusTarget::Dialog | FocusTarget::CommandPalette => true,
    }
}

pub const PANE_FOCUS_TARGETS: [FocusTarget; 3] = [
    FocusTarget::Navigator,
    FocusTarget::Workspace,
    FocusTarget::Inspector,
];

pub fn pane_focus_targets(app: &crate::App) -> impl Iterator<Item = FocusTarget> + '_ {
    PANE_FOCUS_TARGETS
        .into_iter()
        .filter(|target| focus_target_is_relevant(app, *target))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceSubfocus {
    Main,
    Secondary,
    Context,
}

impl WorkspaceSubfocus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Main => "Main",
            Self::Secondary => "Secondary",
            Self::Context => "Context",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InspectorSubfocus {
    Facts,
    Output,
    Actions,
}

impl InspectorSubfocus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Facts => "Facts",
            Self::Output => "Output",
            Self::Actions => "Actions",
        }
    }
}

pub const fn workspace_subfocus_count(screen: Screen) -> usize {
    match screen {
        Screen::Tasks => 3,
        Screen::Logs
        | Screen::Recipes
        | Screen::Devtool
        | Screen::Layers
        | Screen::Packages
        | Screen::Images
        | Screen::Sdk
        | Screen::Testing
        | Screen::Security
        | Screen::Qa
        | Screen::Maintenance => 2,
        _ => 1,
    }
}

pub const fn pane_focus_label(
    focus: FocusTarget,
    workspace: WorkspaceSubfocus,
    inspector: InspectorSubfocus,
) -> &'static str {
    match focus {
        FocusTarget::Navigator => "Navigator/Tree",
        FocusTarget::Workspace => match workspace {
            WorkspaceSubfocus::Main => "Workspace/Main",
            WorkspaceSubfocus::Secondary => "Workspace/Secondary",
            WorkspaceSubfocus::Context => "Workspace/Context",
        },
        FocusTarget::Inspector => match inspector {
            InspectorSubfocus::Facts => "Inspector/Facts",
            InspectorSubfocus::Output => "Inspector/Output",
            InspectorSubfocus::Actions => "Inspector/Actions",
        },
        FocusTarget::Dialog => "Dialog",
        FocusTarget::CommandPalette => "Command Palette",
    }
}

#[cfg(test)]
#[path = "tests/focus/mod.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/focus_inward_navigation/mod.rs"]
mod inward_navigation_tests;
