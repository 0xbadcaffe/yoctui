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
    if app.screen == Screen::RawMode && app.raw_mode.modal_active() { return None; }
    if app.terminal.mode == yoctui_model::TerminalWorkbenchMode::KillConfirmation {
        return None;
    }
    if !app.preferences.mouse_enabled {
        return None;
    }
    if app.menu.is_open() {
        return application_menu_mouse_action(mouse, app, terminal_width, terminal_height);
    }
    if app.active_dialog().is_some() {
        return dialog_mouse_action(mouse, app, terminal_width, terminal_height);
    }
    if app.screen == Screen::Workbench
        && crate::workbench_modal_open(app)
    {
        let shell = workbench_shell(app, terminal_width, terminal_height)?;
        let region = workbench_mouse_region(mouse, app, shell)?;
        return (region.target == FocusTarget::Workspace)
            .then(|| development_mouse_action(mouse, app, region.area))
            .flatten();
    }
    if matches!(mouse.kind, MouseKind::ContextDown) {
        return Some(Action::OpenContextMenu);
    }
    if let Some(zoomed) = app.zoomed_pane
        && app.screen != Screen::DiffTool
        && !(app.screen == Screen::Hardware && app.hardware.viewer.is_some())
        && !((matches!(app.screen, Screen::TerminalSessions)
            || app.platform_menuconfig_visible())
            && !app.daemon.pty_sessions.is_empty())
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
    if (app.screen == Screen::TerminalSessions || app.platform_menuconfig_visible())
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
            let selection = if region.area.width == 26 && app.screen == Screen::Tasks {
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
        if app.screen == Screen::Workbench { return development_mouse_action(mouse, app, region.area); }
        if app.screen == Screen::DiffTool {
            return file_diff_mouse_action(mouse, app, region.area);
        }
        if matches!(mouse.kind, MouseKind::ScrollUp | MouseKind::ScrollDown) {
            let pages_pdf = app.screen == Screen::Hardware
                && app.hardware.viewer.as_ref().is_some_and(|viewer| {
                    matches!(viewer.document.kind, yoctui_model::HardwareDocumentKind::Pdf | yoctui_model::HardwareDocumentKind::Altium | yoctui_model::HardwareDocumentKind::Expedition)
                        && viewer.presentation == yoctui_model::HardwarePresentation::Page
                        && viewer.zoom_percent <= 100 && !viewer.fit_width
                });
            let key = if matches!(mouse.kind, MouseKind::ScrollUp) {
                if pages_pdf { Input::PageUp } else { Input::Up }
            } else if pages_pdf {
                Input::PageDown
            } else {
                Input::Down
            };
            return workspace_collection_action(app, key);
        }
        if matches!(mouse.kind, MouseKind::Down) {
            if app.screen == Screen::Hardware && app.hardware.viewer.is_some()
                && mouse.row == region.area.bottom().saturating_sub(2)
            {
                return crate::hardware_input::hardware_viewer_toolbar_action(mouse.column.saturating_sub(region.area.x));
            }
            if let Some(action) = workspace_tab_click(app, region.area, mouse) {
                return Some(action);
            }
            if app.screen == Screen::Kernel && app.kernel_debug.visible {
                let capacity = usize::from(region.area.height.saturating_sub(6)).max(1);
                let viewport = yoctui_model::centered_viewport_range(Some(app.kernel_debug.selection), yoctui_model::KernelDebugTool::ALL.len(), capacity);
                let first = region.area.y.saturating_add(3);
                if mouse.row >= first && mouse.row < first.saturating_add(capacity as u16) {
                    let index = viewport.start + usize::from(mouse.row - first);
                    if index < yoctui_model::KernelDebugTool::ALL.len() {
                        return Some(Action::KernelDebug(if index == app.kernel_debug.selection {
                            yoctui_model::KernelDebugAction::OpenSelected
                        } else { yoctui_model::KernelDebugAction::SelectAt(index) }));
                    }
                }
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
            if app.screen == Screen::Packages
                && let Some(action) = package_row_click(app, region.area, mouse)
            {
                return Some(action);
            }
            if app.screen == Screen::Images
                && let Some(action) = image_artifact_row_click(app, region.area, mouse)
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
