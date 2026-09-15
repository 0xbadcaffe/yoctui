//! Mouse.
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
        return None;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MouseRect {
    pub(crate) x: u16,
    pub(crate) y: u16,
    pub(crate) width: u16,
    pub(crate) height: u16,
}

impl MouseRect {
    pub(crate) fn right(self) -> u16 {
        self.x.saturating_add(self.width)
    }

    pub(crate) fn bottom(self) -> u16 {
        self.y.saturating_add(self.height)
    }

    pub(crate) fn contains(self, mouse: MouseInput) -> bool {
        mouse.column >= self.x
            && mouse.column < self.right()
            && mouse.row >= self.y
            && mouse.row < self.bottom()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WorkbenchMouseRegion {
    pub(crate) target: FocusTarget,
    pub(crate) area: MouseRect,
}

/// Shared chrome allocation keeps live mouse input aligned with the renderer.
pub fn workbench_chrome_heights(app: &yoctui_model::App, width: u16, height: u16) -> [u16; 2] {
    let concept = matches!(
        app.screen,
        Screen::Dashboard
            | Screen::Tasks
            | Screen::Errors
            | Screen::Images
            | Screen::Recipes
            | Screen::TerminalSessions
    );
    if (width == 160 && height == 50) || (concept && width >= 150 && height >= 50) {
        [5, 3]
    } else {
        [2, 2]
    }
}

/// Wide pane widths in terminal cells, shared by rendering and hit testing.
pub fn workbench_pane_widths(app: &yoctui_model::App, width: u16, height: u16) -> [u16; 3] {
    let compact = app.preferences.density == yoctui_model::UiDensity::Compact;
    let concept = matches!(
        app.screen,
        Screen::Dashboard
            | Screen::Tasks
            | Screen::Errors
            | Screen::Images
            | Screen::TerminalSessions
    );
    if !concept
        && width == 160
        && height == 50
        && !matches!(app.screen, Screen::Layers | Screen::Recipes)
    {
        return [28, 86, 46];
    }
    let navigator = if compact {
        18
    } else if concept && width >= 150 && height >= 50 {
        ((u32::from(width) * 17 / 100) as u16).clamp(22, 30)
    } else if width == 160 && matches!(app.screen, Screen::Tasks | Screen::Dashboard) {
        26
    } else {
        22
    };
    if matches!(app.screen, Screen::Layers | Screen::Recipes)
        || (app.screen == Screen::TerminalSessions && app.selected_terminal_is_menuconfig())
    {
        return [navigator, width.saturating_sub(navigator), 0];
    }
    let inspector = if concept && width >= 150 && height >= 50 {
        let percent = if app.screen == Screen::TerminalSessions {
            21
        } else {
            29
        };
        ((u32::from(width) * percent / 100) as u16).max(32)
    } else if width == 160 && matches!(app.screen, Screen::Tasks | Screen::Dashboard) {
        45
    } else {
        let percent = if matches!(app.screen, Screen::Tasks | Screen::Dashboard) {
            56
        } else {
            43
        };
        width
            .saturating_sub(navigator)
            .saturating_sub(((u32::from(width) * percent + 50) / 100) as u16)
            .max(28)
    };
    [
        navigator,
        width.saturating_sub(navigator).saturating_sub(inspector),
        inspector,
    ]
}

/// Visible cells available to the selected daemon PTY.
///
/// This mirrors the production shell and Terminal Sessions allocations so a
/// writer can send one exact resize without retaining renderer-owned state.
pub fn terminal_workspace_dimensions(
    app: &yoctui_model::App,
    width: u16,
    height: u16,
) -> Option<yoctui_model::PtyDimensions> {
    if app.screen != Screen::TerminalSessions
        || !app.selected_terminal_is_menuconfig()
        || app.terminal.mode != yoctui_model::TerminalWorkbenchMode::Live
    {
        return None;
    }
    let shell = workbench_shell(app, width, height)?;
    let (workspace_width, workspace_height) = if app.zoomed_pane == Some(FocusTarget::Workspace) {
        (shell.width, shell.height.saturating_sub(1))
    } else if app.zoomed_pane.is_some() {
        return None;
    } else if shell.width >= 130 {
        (
            workbench_pane_widths(app, shell.width, height)[1],
            shell.height,
        )
    } else if shell.width >= 100 {
        let navigator = if app.preferences.density == yoctui_model::UiDensity::Compact {
            18
        } else {
            22
        };
        (shell.width.saturating_sub(navigator), shell.height)
    } else if app.focus == FocusTarget::Workspace {
        (shell.width, shell.height.saturating_sub(1))
    } else {
        return None;
    };
    let prefix_help = 0;
    let terminal_area = MouseRect {
        x: 0,
        y: 0,
        width: workspace_width,
        height: workspace_height.saturating_sub(3 + 1 + prefix_help),
    };
    let mut panes = Vec::new();
    collect_terminal_mouse_panes(&app.pane_layout.root, terminal_area, &mut panes);
    let pane = if panes.len() == 1 {
        panes.first()
    } else {
        panes.get(app.pty_selection)
    }?;
    let columns = pane.0.width.saturating_sub(2).clamp(2, 512);
    let rows = pane.0.height.saturating_sub(2 + 1).clamp(1, 512);
    Some(yoctui_model::PtyDimensions { columns, rows })
}

pub(crate) fn workbench_shell(
    app: &yoctui_model::App,
    width: u16,
    height: u16,
) -> Option<MouseRect> {
    let [header, footer] = workbench_chrome_heights(app, width, height);
    (width >= 80 && height >= 24).then_some(MouseRect {
        x: 0,
        y: header,
        width,
        height: height.saturating_sub(header + footer),
    })
}

pub(crate) fn workbench_mouse_region(
    mouse: MouseInput,
    app: &yoctui_model::App,
    shell: MouseRect,
) -> Option<WorkbenchMouseRegion> {
    if !shell.contains(mouse) {
        return None;
    }
    if let Some(target) = app.zoomed_pane {
        let area = MouseRect {
            y: shell.y + 1,
            height: shell.height.saturating_sub(1),
            ..shell
        };
        return area
            .contains(mouse)
            .then_some(WorkbenchMouseRegion { target, area });
    }
    if shell.width >= 130 {
        let total_height = shell.height + if shell.y == 5 { 8 } else { 4 };
        let [navigator_width, workspace_width, _] =
            workbench_pane_widths(app, shell.width, total_height);
        let navigator = MouseRect {
            width: navigator_width,
            ..shell
        };
        let workspace = MouseRect {
            x: shell.x + navigator_width,
            width: workspace_width,
            ..shell
        };
        let inspector = MouseRect {
            x: workspace.right(),
            width: shell.right().saturating_sub(workspace.right()),
            ..shell
        };
        return [
            WorkbenchMouseRegion {
                target: FocusTarget::Navigator,
                area: navigator,
            },
            WorkbenchMouseRegion {
                target: FocusTarget::Workspace,
                area: workspace,
            },
            WorkbenchMouseRegion {
                target: FocusTarget::Inspector,
                area: inspector,
            },
        ]
        .into_iter()
        .find(|region| region.area.contains(mouse));
    }
    if shell.width >= 100 {
        let navigator = MouseRect {
            width: if app.preferences.density == yoctui_model::UiDensity::Compact {
                18
            } else {
                22
            },
            ..shell
        };
        if navigator.contains(mouse) {
            return Some(WorkbenchMouseRegion {
                target: FocusTarget::Navigator,
                area: navigator,
            });
        }
        return Some(WorkbenchMouseRegion {
            target: if app.focus == FocusTarget::Inspector {
                FocusTarget::Inspector
            } else {
                FocusTarget::Workspace
            },
            area: MouseRect {
                x: navigator.right(),
                width: shell.right().saturating_sub(navigator.right()),
                ..shell
            },
        });
    }
    if mouse.row == shell.y {
        return narrow_switcher_target(app, mouse.column).map(|target| WorkbenchMouseRegion {
            target,
            area: shell,
        });
    }
    Some(WorkbenchMouseRegion {
        target: match app.focus {
            FocusTarget::Navigator => FocusTarget::Navigator,
            FocusTarget::Inspector => FocusTarget::Inspector,
            FocusTarget::Workspace | FocusTarget::Dialog | FocusTarget::CommandPalette => {
                FocusTarget::Workspace
            }
        },
        area: MouseRect {
            y: shell.y + 1,
            height: shell.height.saturating_sub(1),
            ..shell
        },
    })
}

pub(crate) fn narrow_switcher_target(app: &yoctui_model::App, column: u16) -> Option<FocusTarget> {
    let mut cursor = "Panes: ".len() as u16;
    for target in yoctui_model::pane_focus_targets(app) {
        let name = target.label();
        let width = name.len() as u16 + u16::from(app.focus == target) * 2;
        if (cursor..cursor.saturating_add(width)).contains(&column) {
            return Some(target);
        }
        cursor = cursor.saturating_add(width + 2);
    }
    None
}

pub(crate) fn dialog_mouse_action(
    mouse: MouseInput,
    app: &yoctui_model::App,
    terminal_width: u16,
    terminal_height: u16,
) -> Option<Action> {
    if matches!(mouse.kind, MouseKind::Down | MouseKind::Drag)
        && let Some(editor) = active_dialog_textarea(app)
    {
        let width = 110u16.min(terminal_width.saturating_sub(2)).max(1);
        let preferred_height = terminal_height.saturating_sub(4).min(34);
        let height = preferred_height
            .min(terminal_height.saturating_sub(2))
            .max(1);
        let x = terminal_width.saturating_sub(width) / 2;
        let y = terminal_height.saturating_sub(height) / 2;
        let content_y = y.saturating_add(2);
        let gutter = editor.line_count().to_string().len() as u16 + 3;
        let content_x = x.saturating_add(1).saturating_add(gutter);
        if mouse.row >= content_y
            && mouse.row < y.saturating_add(height).saturating_sub(3)
            && mouse.column >= content_x
            && mouse.column < x.saturating_add(width).saturating_sub(1)
        {
            return Some(Action::EditActivePopup(
                PopupEditorCommand::SelectPosition {
                    line: usize::from(mouse.row - content_y),
                    column: usize::from(mouse.column - content_x),
                    extend: mouse.kind == MouseKind::Drag,
                },
            ));
        }
    }
    if matches!(mouse.kind, MouseKind::Down | MouseKind::ContextDown) {
        return Some(Action::Focus(FocusTarget::Dialog));
    }
    let delta = match mouse.kind {
        MouseKind::ScrollUp => -1,
        MouseKind::ScrollDown => 1,
        MouseKind::Drag | MouseKind::Up | MouseKind::Down | MouseKind::ContextDown => return None,
    };
    match app.active_dialog()? {
        yoctui_model::Dialog::EnvironmentSetup(setup) if setup.editor.is_none() => {
            environment_setup_action(setup, if delta < 0 { Input::Up } else { Input::Down })
        }
        yoctui_model::Dialog::ThemePicker { .. } => Some(Action::SelectTheme { delta }),
        yoctui_model::Dialog::ImagePicker(_) => Some(Action::SelectImage { delta }),
        yoctui_model::Dialog::RecipeTaskPicker(_) => Some(Action::SelectRecipeTask { delta }),
        yoctui_model::Dialog::RecipeTaskLogPicker(_) => Some(Action::SelectRecipeTaskLog { delta }),
        yoctui_model::Dialog::RecipePatchPicker(_) => Some(Action::SelectRecipePatch { delta }),
        yoctui_model::Dialog::SignatureTaskPicker(_) => Some(Action::SelectSignatureTask { delta }),
        yoctui_model::Dialog::ConfigSourcePicker(_) => Some(Action::SelectConfigSource { delta }),
        yoctui_model::Dialog::ConfigScopePicker(_) => Some(Action::SelectConfigScope { delta }),
        yoctui_model::Dialog::DevtoolFinishPicker(_) => {
            Some(Action::SelectDevtoolFinishLayer { delta })
        }
        yoctui_model::Dialog::WicDevicePicker(_) => Some(Action::SelectWicDevice { delta }),
        _ => None,
    }
}

pub(crate) fn active_dialog_textarea(
    app: &yoctui_model::App,
) -> Option<&yoctui_model::TextAreaState> {
    match app.active_dialog()? {
        yoctui_model::Dialog::BuildEnvironmentEditor(editor)
        | yoctui_model::Dialog::BuildEnvironmentCloneEditor(editor)
        | yoctui_model::Dialog::BbmaskEdit(editor)
        | yoctui_model::Dialog::SdkPublishTomlEditor(editor)
        | yoctui_model::Dialog::SdkNativeTomlEditor(editor) => Some(editor),
        yoctui_model::Dialog::ConfigEdit { editor, .. }
        | yoctui_model::Dialog::BuildTarget { editor, .. } => Some(editor),
        yoctui_model::Dialog::WicCreateTomlEditor { editor, .. }
        | yoctui_model::Dialog::TestLaunchTomlEditor { editor, .. }
        | yoctui_model::Dialog::TestResultImportTomlEditor { editor, .. }
        | yoctui_model::Dialog::TestComparisonTomlEditor { editor, .. }
        | yoctui_model::Dialog::TestJunitTomlEditor { editor, .. } => Some(editor),
        yoctui_model::Dialog::Security(yoctui_model::SecurityDialog::Import { editor, .. })
        | yoctui_model::Dialog::Qa(yoctui_model::QaDialog::Import { editor, .. }) => Some(editor),
        yoctui_model::Dialog::Maintenance(dialog) => match dialog.as_ref() {
            yoctui_model::MaintenanceDialog::ReadinessToml { editor, .. }
            | yoctui_model::MaintenanceDialog::CleanupToml { editor, .. }
            | yoctui_model::MaintenanceDialog::PrServiceToml { editor, .. }
            | yoctui_model::MaintenanceDialog::LockedCacheToml { editor, .. }
            | yoctui_model::MaintenanceDialog::BuildHistoryToml { editor, .. }
            | yoctui_model::MaintenanceDialog::GitArchiveToml { editor, .. } => Some(editor),
            _ => None,
        },
        _ => None,
    }
}

pub fn workspace_collection_action(app: &yoctui_model::App, key: Input) -> Option<Action> {
    let delta = collection_scroll_delta(key)?;
    match app.screen {
        Screen::Dashboard | Screen::Tasks => tasks_action(app.task_filter_editing, key),
        Screen::Insights => None,
        Screen::BuildHistory => Some(Action::SelectBuildHistory { delta }),
        Screen::Dependencies => dependency_workspace_action(app.dependency_graph_searching, key),
        Screen::Signatures => signature_workspace_action(key),
        Screen::Recipes => recipes_workspace_action(app.metadata_searching, key),
        Screen::Packages => package_workspace_action(app.package_searching, key),
        Screen::Images => {
            images_workspace_action_for_view(app.image_artifact_searching, app.images_view, key)
        }
        Screen::Kernel => platform_workspace_action(key),
        Screen::Firmware => firmware_workspace_action(key),
        Screen::Sdk => sdk_workspace_action(app.sdk_artifact_searching, key),
        Screen::Testing => match app.test_view {
            TestWorkspaceView::Launches => testing_workspace_action(key),
            TestWorkspaceView::Results => test_results_workspace_action(
                app.test_result_searching,
                app.test_result_drilled,
                key,
            ),
            TestWorkspaceView::Comparison => test_comparison_workspace_action(key),
        },
        Screen::Security => security_workspace_action(
            app.security.view,
            app.security.drilled,
            app.security.searching,
            key,
        ),
        Screen::Qa => qa_workspace_action(app.qa.view, app.qa.drilled, app.qa.searching, key),
        Screen::Layers => {
            if app.layer_browser.is_some() {
                layer_tree_action(app.metadata_searching, key)
            } else {
                Some(Action::SelectLayer { delta })
            }
        }
        Screen::Configuration => config_workspace_action(app.metadata_searching, key),
        Screen::RawMode => raw_mode_input(app, key).map(Action::RawMode),
        Screen::TerminalSessions => terminal_workspace_action(app, key),
        Screen::Maintenance => maintenance_workspace_action(
            app.maintenance.view,
            match app.maintenance.view {
                MaintenanceView::Sstate => 2,
                MaintenanceView::Services => 1,
                MaintenanceView::Release | MaintenanceView::Integrations => 4,
            },
            key,
        ),
        Screen::Logs => log_workspace_action(app, key),
        Screen::Errors => errors_action(key),
        Screen::BuildEnvironment => build_environment_action(key),
        Screen::Compatibility => {
            compatibility_ui_inspector_action(app.compatibility_ui.searching, key)
        }
        Screen::Settings => settings_action(key),
        Screen::LayerRelationships | Screen::Bbmask | Screen::Help => None,
    }
}

pub fn platform_workspace_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectKernelFile { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectKernelFile { delta: 1 }),
        Input::PageUp => Some(Action::SelectKernelFile { delta: -10 }),
        Input::PageDown => Some(Action::SelectKernelFile { delta: 10 }),
        Input::Tab | Input::BackTab => Some(Action::CycleKernelView),
        Input::Char('m') => Some(Action::LaunchKernelMenuconfig),
        Input::Enter | Input::Char('e') => Some(Action::OpenSelectedKernelFile),
        Input::Char('o') => Some(Action::ExploreSelectedKernelRoot),
        Input::Char('c') => Some(Action::CompileSelectedKernelDts),
        Input::Char('d') => Some(Action::DecompileSelectedKernelDtb),
        Input::Char('r') => Some(Action::InspectKernel),
        _ => None,
    }
}

pub fn firmware_workspace_action(key: Input) -> Option<Action> {
    match key {
        Input::Up | Input::Char('k') => Some(Action::SelectFirmwareFile { delta: -1 }),
        Input::Down | Input::Char('j') => Some(Action::SelectFirmwareFile { delta: 1 }),
        Input::PageUp => Some(Action::SelectFirmwareFile { delta: -10 }),
        Input::PageDown => Some(Action::SelectFirmwareFile { delta: 10 }),
        Input::Tab | Input::BackTab => Some(Action::CycleFirmwareView),
        Input::Char('m') => Some(Action::LaunchFirmwareMenuconfig),
        Input::Enter | Input::Char('e') => Some(Action::OpenSelectedFirmwareFile),
        Input::Char('o') => Some(Action::ExploreSelectedFirmwareRoot),
        Input::Char('c') => Some(Action::CompileSelectedFirmwareDts),
        Input::Char('d') => Some(Action::DecompileSelectedFirmwareDtb),
        Input::Char('r') => Some(Action::InspectFirmware),
        _ => None,
    }
}

pub fn dashboard_workspace_action(key: Input) -> Option<Action> {
    match key {
        Input::Char('f') => Some(Action::OpenRawFavorites),
        Input::Char('t') => Some(Action::Open(Screen::TerminalSessions)),
        _ => None,
    }
}

pub fn overview_workspace_action(key: Input) -> Option<Action> {
    match key {
        Input::Left | Input::Char('[') | Input::Char('h') => {
            Some(Action::ShiftOverviewView { delta: -1 })
        }
        Input::Right | Input::Char(']') | Input::Char('l') => {
            Some(Action::ShiftOverviewView { delta: 1 })
        }
        Input::Char(character @ '1'..='8') => {
            yoctui_model::OverviewView::from_number(character as u8 - b'0')
                .map(Action::SelectOverviewView)
        }
        _ => None,
    }
}

pub(crate) fn workspace_tab_click(
    app: &yoctui_model::App,
    area: MouseRect,
    mouse: MouseInput,
) -> Option<Action> {
    if mouse.row != area.y.saturating_add(1) || mouse.column <= area.x {
        return None;
    }
    let column = mouse.column - area.x - 1;
    match app.screen {
        Screen::Security if column < 6 => (app.security.view != SecurityView::Cves)
            .then_some(Action::Security(SecurityAction::CycleView)),
        Screen::Security if (9..15).contains(&column) => (app.security.view != SecurityView::Sbom)
            .then_some(Action::Security(SecurityAction::CycleView)),
        Screen::Qa if column < 17 => {
            (app.qa.view != QaView::RecipeKernel).then_some(Action::Qa(QaAction::CycleView))
        }
        Screen::Qa if (20..30).contains(&column) => {
            (app.qa.view != QaView::LayerQa).then_some(Action::Qa(QaAction::CycleView))
        }
        Screen::Testing if column < 10 => Some(Action::SelectTestView(TestWorkspaceView::Launches)),
        Screen::Testing if (13..22).contains(&column) => {
            Some(Action::SelectTestView(TestWorkspaceView::Results))
        }
        Screen::Testing if (25..37).contains(&column) => {
            Some(Action::SelectTestView(TestWorkspaceView::Comparison))
        }
        Screen::Logs if column < 16 => (app.log_workspace_view
            != yoctui_model::LogWorkspaceView::BitBake)
            .then_some(Action::CycleLogWorkspaceView),
        Screen::Logs if (18..42).contains(&column) => (app.log_workspace_view
            != yoctui_model::LogWorkspaceView::Yoctui)
            .then_some(Action::CycleLogWorkspaceView),
        _ => None,
    }
}

/// Shared table/log/history/telemetry row allocation for rendering and mouse hit testing.
pub fn task_workspace_panel_heights(app: &yoctui_model::App, width: u16, height: u16) -> [u16; 4] {
    if width >= 64 && height == 42 {
        return [12, 12, 10, 8];
    }
    if width == 89 && height == 44 {
        return [17, 14, 9, 4];
    }
    if height >= 46 && width >= 64 {
        let filesystem_known = app.workspace.build_dir.is_some()
            && matches!((app.host_telemetry.disk_total_bytes, app.host_telemetry.disk_available_bytes),
                (Some(total), Some(available)) if total > 0 && available <= total);
        let telemetry_known = filesystem_known
            || app
                .host_telemetry_projection()
                .series
                .iter()
                .any(|series| series.is_supported());
        if telemetry_known {
            let extra = height - 46;
            return [14 + extra.div_ceil(2), 14 + extra / 2, 10, 8];
        }
    }
    let telemetry = height.min(4);
    let content = height - telemetry;
    if content >= 27 {
        let main = (u32::from(content) * 45 / 100) as u16;
        let log = (u32::from(content) * 30 / 100) as u16;
        [main, log, content - main - log, telemetry]
    } else if content >= 14 {
        let log = (content * 38 / 100).max(6);
        [content - log, log, 0, telemetry]
    } else {
        [content, 0, 0, telemetry]
    }
}

pub(crate) fn task_row_click(
    app: &yoctui_model::App,
    area: MouseRect,
    mouse: MouseInput,
) -> Option<Action> {
    let table_height = task_workspace_panel_heights(app, area.width, area.height)[0];
    let summary_height = if app.screen == Screen::Dashboard {
        4
    } else {
        2
    };
    let first_row = area.y.saturating_add(summary_height + 2);
    let visible_rows = usize::from(table_height.saturating_sub(summary_height + 3));
    if mouse.row < first_row || mouse.row >= first_row.saturating_add(visible_rows as u16) {
        return None;
    }
    let rows = app.visible_task_row_refs_at(SystemTime::now());
    let selected = app.task_progress_scroll.min(rows.len().saturating_sub(1));
    let viewport_start = selected
        .saturating_sub(visible_rows / 2)
        .min(rows.len().saturating_sub(visible_rows));
    let clicked = viewport_start + usize::from(mouse.row - first_row);
    (clicked < rows.len()).then_some(Action::ScrollBuildTasks {
        delta: clicked as isize - app.task_progress_scroll as isize,
    })
}

pub(crate) fn dependency_row_click(
    app: &yoctui_model::App,
    area: MouseRect,
    mouse: MouseInput,
) -> Option<Action> {
    let graph = app.dependency_graph.graph()?;
    let anchor = app.dependency_graph_anchor.as_ref().unwrap_or(&graph.root);
    let projection = graph.project(
        anchor,
        app.dependency_graph_reverse,
        &app.dependency_graph_query,
        &app.dependency_graph_collapsed,
        64,
        8_192,
    );
    let first_row = area.y.saturating_add(2);
    let visible_rows = usize::from(area.height.saturating_sub(3)).max(1);
    if mouse.row < first_row || mouse.row >= first_row.saturating_add(visible_rows as u16) {
        return None;
    }
    let selected = app
        .dependency_graph_selection
        .as_ref()
        .and_then(|identity| projection.rows.iter().position(|row| &row.id == identity))
        .unwrap_or(0);
    let viewport_start = selected.saturating_add(1).saturating_sub(visible_rows);
    let clicked = viewport_start + usize::from(mouse.row - first_row);
    projection
        .rows
        .get(clicked)
        .map(|row| Action::SelectDependencyGraphNodeAt {
            identity: row.id.clone(),
        })
}

pub(crate) fn terminal_session_mouse_action(
    mouse: MouseInput,
    app: &yoctui_model::App,
    shell: MouseRect,
) -> Option<Action> {
    if !shell.contains(mouse) {
        return None;
    }
    let footer = if !app.selected_terminal_is_menuconfig() && shell.height >= 30 {
        3
    } else {
        0
    };
    let shell = MouseRect {
        y: shell.y + 4,
        height: shell.height.saturating_sub(4 + footer),
        ..shell
    };
    if !shell.contains(mouse) {
        return None;
    }
    match mouse.kind {
        MouseKind::Down => {
            let mut leaves = Vec::new();
            collect_terminal_mouse_panes(&app.pane_layout.root, shell, &mut leaves);
            leaves
                .into_iter()
                .enumerate()
                .find(|(index, (area, _))| {
                    *index < app.daemon.pty_sessions.len() && area.contains(mouse)
                })
                .map(|(index, (_, pane))| Action::SelectPtyPane { pane, index })
        }
        MouseKind::ContextDown => None,
        MouseKind::Drag => terminal_resize_action(mouse, app, shell),
        MouseKind::ScrollUp => Some(Action::SelectPtySession { delta: -1 }),
        MouseKind::ScrollDown => Some(Action::SelectPtySession { delta: 1 }),
        MouseKind::Up => None,
    }
}

pub(crate) fn collect_terminal_mouse_panes(
    node: &yoctui_model::PaneNode,
    area: MouseRect,
    output: &mut Vec<(MouseRect, yoctui_model::PaneId)>,
) {
    match node {
        yoctui_model::PaneNode::Leaf { id } => output.push((area, *id)),
        yoctui_model::PaneNode::Split {
            axis,
            ratio_per_mille,
            first,
            second,
        } => {
            let (first_area, second_area) = split_mouse_rect(area, *axis, *ratio_per_mille);
            collect_terminal_mouse_panes(first, first_area, output);
            collect_terminal_mouse_panes(second, second_area, output);
        }
    }
}

pub(crate) fn split_mouse_rect(
    area: MouseRect,
    axis: SplitAxis,
    ratio_per_mille: u16,
) -> (MouseRect, MouseRect) {
    let ratio = f32::from(ratio_per_mille) / 1000.0;
    match axis {
        SplitAxis::Horizontal => {
            let first_width = ((f32::from(area.width) * ratio) as u16)
                .max(1)
                .min(area.width.saturating_sub(1));
            (
                MouseRect {
                    width: first_width,
                    ..area
                },
                MouseRect {
                    x: area.x + first_width,
                    width: area.width - first_width,
                    ..area
                },
            )
        }
        SplitAxis::Vertical => {
            let first_height = ((f32::from(area.height) * ratio) as u16)
                .max(1)
                .min(area.height.saturating_sub(1));
            (
                MouseRect {
                    height: first_height,
                    ..area
                },
                MouseRect {
                    y: area.y + first_height,
                    height: area.height - first_height,
                    ..area
                },
            )
        }
    }
}

pub(crate) fn terminal_resize_action(
    mouse: MouseInput,
    app: &yoctui_model::App,
    shell: MouseRect,
) -> Option<Action> {
    let (axis, ratio, area) =
        focused_split_geometry(&app.pane_layout.root, app.pane_layout.focused, shell)?;
    let desired = match axis {
        SplitAxis::Horizontal => {
            u32::from(mouse.column.saturating_sub(area.x)).saturating_mul(1000)
                / u32::from(area.width.max(1))
        }
        SplitAxis::Vertical => {
            u32::from(mouse.row.saturating_sub(area.y)).saturating_mul(1000)
                / u32::from(area.height.max(1))
        }
    }
    .clamp(100, 900) as i16;
    let delta = desired - ratio as i16;
    (delta != 0).then_some(Action::ResizeFocusedPane {
        delta_per_mille: delta,
    })
}

pub(crate) fn focused_split_geometry(
    node: &yoctui_model::PaneNode,
    focused: yoctui_model::PaneId,
    area: MouseRect,
) -> Option<(SplitAxis, u16, MouseRect)> {
    let yoctui_model::PaneNode::Split {
        axis,
        ratio_per_mille,
        first,
        second,
    } = node
    else {
        return None;
    };
    let (first_area, second_area) = split_mouse_rect(area, *axis, *ratio_per_mille);
    if matches!(first.as_ref(), yoctui_model::PaneNode::Leaf { id } if *id == focused)
        || matches!(second.as_ref(), yoctui_model::PaneNode::Leaf { id } if *id == focused)
    {
        return Some((*axis, *ratio_per_mille, area));
    }
    if pane_node_contains(first, focused) {
        focused_split_geometry(first, focused, first_area)
    } else if pane_node_contains(second, focused) {
        focused_split_geometry(second, focused, second_area)
    } else {
        None
    }
}

pub(crate) fn pane_node_contains(
    node: &yoctui_model::PaneNode,
    pane: yoctui_model::PaneId,
) -> bool {
    match node {
        yoctui_model::PaneNode::Leaf { id } => *id == pane,
        yoctui_model::PaneNode::Split { first, second, .. } => {
            pane_node_contains(first, pane) || pane_node_contains(second, pane)
        }
    }
}

pub(crate) fn literal_navigator_selection_at_row(
    app: &yoctui_model::App,
    row: usize,
) -> Option<usize> {
    let layer_rows = app.workspace.layers.len().clamp(1, 7);
    let recipe_rows = app.workspace.recipes.len().clamp(1, 3);
    let image_rows = app.available_images.len().clamp(1, 2);
    let layers_end = 1 + layer_rows;
    if row < layers_end {
        return Some(1);
    }
    let recipes_start = layers_end;
    let recipes_end = recipes_start + 1 + recipe_rows;
    if row < recipes_end {
        return Some(2);
    }
    let images_start = recipes_end;
    let images_end = images_start + 1 + image_rows;
    if row < images_end {
        return Some(4);
    }
    let tasks_start = images_end;
    if row == tasks_start {
        return Some(6);
    }
    const TASK_DESTINATIONS: [usize; 8] = [6, 11, 13, 14, 15, 5, 12, 16];
    TASK_DESTINATIONS.get(row - tasks_start - 1).copied()
}
