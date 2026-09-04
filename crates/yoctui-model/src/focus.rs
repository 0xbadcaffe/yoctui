use crate::{FocusTarget, Screen};

pub fn focus_target_is_relevant(app: &crate::App, target: FocusTarget) -> bool {
    match target {
        FocusTarget::Navigator => true,
        FocusTarget::Workspace => match app.screen {
            // The Dashboard is a read-only cockpit. Its task, log, history,
            // telemetry, and status panels never become actionable merely
            // because live or retained build data appears.
            Screen::Dashboard => false,
            _ => true,
        },
        // Layers and Recipes carry their selectable preview inside the workspace.
        // An idle Dashboard inspector is read-only status and must not become a
        // keyboard focus trap.
        FocusTarget::Inspector => !matches!(
            app.screen,
            Screen::Dashboard | Screen::Layers | Screen::Recipes
        ),
        FocusTarget::Dialog | FocusTarget::CommandPalette => true,
    }
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
mod tests {
    use super::*;
    use crate::{Action, App, CommandId, update};

    #[test]
    fn dashboard_navigator_focus_always_skips_read_only_panes() {
        let mut app = App::new(32, 8_192);
        assert_eq!(app.screen, Screen::Dashboard);
        let _ = update(&mut app, Action::Focus(FocusTarget::Navigator));
        assert_eq!(app.focus, FocusTarget::Navigator);

        let _ = update(&mut app, Action::CycleFocus { backwards: false });
        assert_eq!(app.focus, FocusTarget::Navigator);

        app.tasks.insert(
            crate::TaskId("busybox:do_compile".into()),
            crate::TaskInfo::active(
                crate::TaskId("busybox:do_compile".into()),
                "busybox".into(),
                "do_compile".into(),
            ),
        );
        let _ = update(&mut app, Action::CycleFocus { backwards: false });
        assert_eq!(app.focus, FocusTarget::Navigator);

        app.completed_tasks.push_back(crate::CompletedTask {
            task: crate::TaskInfo::active(
                crate::TaskId("bash:do_compile".into()),
                "bash".into(),
                "do_compile".into(),
            ),
            success: true,
        });
        let _ = update(&mut app, Action::CycleFocus { backwards: true });
        assert_eq!(app.focus, FocusTarget::Navigator);
        let _ = update(&mut app, Action::Focus(FocusTarget::Workspace));
        assert_eq!(app.focus, FocusTarget::Navigator);

        app.navigator_selection = 0;
        let _ = update(&mut app, Action::ActivateNavigator);
        assert_eq!(app.screen, Screen::Dashboard);
        assert_eq!(app.focus, FocusTarget::Navigator);
    }

    #[test]
    fn ux_focus_subfocus_zoom_palette_and_modal_restore_preserve_client_state() {
        let mut app = App::new(32, 8_192);
        app.screen = Screen::Tasks;
        app.focus = FocusTarget::Workspace;
        app.task_progress_scroll = 7;
        app.logs.scroll_offset = 11;
        let selections = (app.task_progress_scroll, app.logs.scroll_offset);

        let _ = update(&mut app, Action::CyclePaneSubfocus { backwards: false });
        assert_eq!(app.workspace_subfocus, WorkspaceSubfocus::Secondary);
        assert_eq!(app.pane_focus_label(), "Workspace/Secondary");
        let _ = update(&mut app, Action::TogglePaneZoom);
        assert_eq!(app.zoomed_pane, Some(FocusTarget::Workspace));

        let _ = update(&mut app, Action::OpenCommandPalette);
        assert_eq!(app.focus, FocusTarget::CommandPalette);
        assert_eq!(app.focus_return, Some(FocusTarget::Workspace));
        let _ = update(&mut app, Action::CloseCommandPalette);
        assert_eq!(app.focus, FocusTarget::Workspace);
        assert_eq!(app.zoomed_pane, Some(FocusTarget::Workspace));
        assert_eq!(
            (app.task_progress_scroll, app.logs.scroll_offset),
            selections
        );

        let _ = update(&mut app, Action::OpenCommandPalette);
        app.command_palette_query = "focus inspector".into();
        app.command_palette_selection = 0;
        let _ = update(&mut app, Action::ActivateCommandPalette);
        assert_eq!(app.focus, FocusTarget::Inspector);
        assert_eq!(app.zoomed_pane, Some(FocusTarget::Inspector));
        assert_eq!(app.inspector_subfocus, InspectorSubfocus::Facts);
        assert_eq!(
            crate::command_action(&app, CommandId::NextSubfocus),
            Action::CyclePaneSubfocus { backwards: false }
        );

        let _ = update(&mut app, Action::TogglePaneZoom);
        assert!(app.zoomed_pane.is_none());
        assert_eq!(
            (app.task_progress_scroll, app.logs.scroll_offset),
            selections
        );
    }
}
