use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseKind {
    Down,
    ContextDown,
    Drag,
    Up,
    ScrollUp,
    ScrollDown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseInput {
    pub kind: MouseKind,
    pub column: u16,
    pub row: u16,
}

pub fn mouse_action(mouse: MouseInput, terminal_width: u16) -> Option<Action> {
    match mouse.kind {
        MouseKind::ContextDown => Some(Action::OpenContextMenu),
        MouseKind::ScrollUp if mouse.column < 24 => Some(Action::SelectNavigator { delta: -1 }),
        MouseKind::ScrollDown if mouse.column < 24 => Some(Action::SelectNavigator { delta: 1 }),
        MouseKind::ScrollUp | MouseKind::ScrollDown => Some(Action::ScrollLogs {
            delta: if matches!(mouse.kind, MouseKind::ScrollUp) {
                1
            } else {
                -1
            },
        }),
        MouseKind::Down if mouse.column < 24 => Some(Action::Focus(FocusTarget::Navigator)),
        MouseKind::Down
            if terminal_width >= 130 && mouse.column >= terminal_width.saturating_sub(28) =>
        {
            Some(Action::Focus(FocusTarget::Inspector))
        }
        MouseKind::Down | MouseKind::Drag => Some(Action::Focus(FocusTarget::Workspace)),
        MouseKind::Up => None,
    }
}

/// Resolve clicks against the active workbench context. Coordinate ownership
/// stays in the app layer so widgets remain render-only and dialogs can trap
/// focus before any workspace action is emitted.
pub fn mouse_action_for_app(
    mouse: MouseInput,
    app: &yoctui_model::App,
    terminal_width: u16,
    terminal_height: u16,
) -> Option<Action> {
    if !app.preferences.mouse_enabled {
        return None;
    }
    if app.menu.is_open() {
        return application_menu_mouse_action(mouse, app, terminal_width, terminal_height);
    }
    if app.active_dialog().is_some() {
        return dialog_mouse_action(mouse, app, terminal_width, terminal_height);
    }
    if matches!(mouse.kind, MouseKind::ContextDown) {
        return Some(Action::OpenContextMenu);
    }
    if let Some(zoomed) = app.zoomed_pane
        && !(app.screen == Screen::TerminalSessions && !app.daemon.pty_sessions.is_empty())
        && matches!(mouse.kind, MouseKind::Down)
    {
        return Some(Action::Focus(zoomed));
    }
    if app.screen == Screen::RawMode
        && let Some(action) = raw_mouse_action(mouse, app, terminal_width, terminal_height)
    {
        return Some(Action::RawMode(action));
    }
    if app.screen == Screen::RawMode
        && matches!(
            app.raw_mode.view,
            yoctui_model::RawModeView::Form | yoctui_model::RawModeView::Preview
        )
    {
        return matches!(mouse.kind, MouseKind::Down).then_some(Action::Focus(FocusTarget::Dialog));
    }
    let shell = workbench_shell(app, terminal_width, terminal_height)?;
    let region = workbench_mouse_region(mouse, app, shell)?;
    if !yoctui_model::focus_target_is_relevant(app, region.target) {
        return None;
    }
    if app.screen == Screen::TerminalSessions
        && !app.daemon.pty_sessions.is_empty()
        && region.target == FocusTarget::Workspace
    {
        return terminal_session_mouse_action(mouse, app, region.area);
    }
    if region.target == FocusTarget::Navigator {
        if matches!(mouse.kind, MouseKind::ScrollUp) {
            return Some(Action::SelectNavigator { delta: -1 });
        }
        if matches!(mouse.kind, MouseKind::ScrollDown) {
            return Some(Action::SelectNavigator { delta: 1 });
        }
        if matches!(mouse.kind, MouseKind::Down)
            && mouse.row > region.area.y
            && mouse.row < region.area.bottom().saturating_sub(1)
        {
            let row = usize::from(mouse.row - region.area.y - 1);
            let selection = if terminal_width == 160 && app.screen == Screen::Tasks {
                literal_navigator_selection_at_row(app, row)
            } else {
                let visible_rows = usize::from(region.area.height.saturating_sub(2));
                let visual_row = app.navigator_viewport_start(visible_rows) + row;
                if let Some(group) = app.navigator_group_at_visual_row(visual_row) {
                    return Some(Action::ToggleNavigatorGroup { group });
                }
                app.navigator_selection_at_visual_row(visual_row)
            };
            if let Some(index) = selection {
                return if app.focus == FocusTarget::Navigator && app.navigator_selection == index {
                    Some(Action::ActivateNavigator)
                } else {
                    Some(Action::SelectNavigatorAt { index })
                };
            }
        }
        if matches!(mouse.kind, MouseKind::Down) {
            return Some(Action::Focus(FocusTarget::Navigator));
        }
        return None;
    }
    if region.target == FocusTarget::Workspace {
        if matches!(mouse.kind, MouseKind::ScrollUp | MouseKind::ScrollDown) {
            let key = if matches!(mouse.kind, MouseKind::ScrollUp) {
                Input::Up
            } else {
                Input::Down
            };
            return workspace_collection_action(app, key);
        }
        if matches!(mouse.kind, MouseKind::Down) {
            if let Some(action) = workspace_tab_click(app, region.area, mouse) {
                return Some(action);
            }
            if app.screen == Screen::Tasks
                && let Some(action) = task_row_click(app, region.area, mouse)
            {
                return Some(action);
            }
            if app.screen == Screen::Dependencies
                && let Some(action) = dependency_row_click(app, region.area, mouse)
            {
                return Some(action);
            }
            return Some(Action::Focus(FocusTarget::Workspace));
        }
        return None;
    }
    if matches!(mouse.kind, MouseKind::Down) {
        return Some(Action::Focus(FocusTarget::Inspector));
    }
    None
}

pub(crate) fn raw_mouse_action(
    mouse: MouseInput,
    app: &yoctui_model::App,
    terminal_width: u16,
    terminal_height: u16,
) -> Option<yoctui_model::RawModeAction> {
    use yoctui_model::{RawBrowserColumn, RawModeAction, RawModeView};
    let shell = workbench_shell(app, terminal_width, terminal_height)?;
    if !shell.contains(mouse) {
        return None;
    }
    let delta = match mouse.kind {
        MouseKind::ScrollUp => Some(-1),
        MouseKind::ScrollDown => Some(1),
        _ => None,
    };
    if let Some(delta) = delta {
        return Some(match app.raw_mode.view {
            RawModeView::Favorites => RawModeAction::SelectFavorite { delta },
            RawModeView::History => RawModeAction::SelectHistory { delta },
            _ if app.raw_mode.browser_column == RawBrowserColumn::Categories => {
                RawModeAction::SelectCategory { delta }
            }
            _ => RawModeAction::SelectCommand { delta },
        });
    }
    if !matches!(mouse.kind, MouseKind::Down) {
        return None;
    }
    if app.raw_mode.view == RawModeView::Favorites {
        let row = usize::from(mouse.row.saturating_sub(shell.y + 1));
        let index = row / 4;
        return (index < app.raw_mode.favorites.len()).then_some(RawModeAction::SelectFavorite {
            delta: index as isize - app.raw_mode.favorite_selection as isize,
        });
    }
    if app.raw_mode.view != RawModeView::Browser {
        return None;
    }
    let category_width = if terminal_width >= 100 {
        shell.width.saturating_mul(52) / 100
    } else {
        shell.width
    };
    let column = if terminal_width >= 100 && mouse.column >= shell.x + category_width {
        RawBrowserColumn::Commands
    } else {
        app.raw_mode.browser_column
    };
    let row = usize::from(mouse.row.saturating_sub(shell.y + 1));
    match column {
        RawBrowserColumn::Categories => {
            let categories = yoctui_model::builtin_raw_catalog().browser_categories();
            let current = app
                .raw_mode
                .category
                .as_ref()
                .and_then(|id| categories.iter().position(|item| &item.id == id))
                .unwrap_or(0);
            Some(RawModeAction::SelectCategory {
                delta: row as isize - current as isize,
            })
        }
        RawBrowserColumn::Commands => {
            let commands = app
                .raw_mode
                .visible_commands(yoctui_model::builtin_raw_catalog());
            let current = app
                .raw_mode
                .command
                .as_ref()
                .and_then(|id| commands.iter().position(|item| &item.id == id))
                .unwrap_or(0);
            let target = row / 2;
            Some(RawModeAction::SelectCommand {
                delta: target as isize - current as isize,
            })
        }
    }
}
