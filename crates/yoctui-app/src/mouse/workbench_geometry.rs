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
