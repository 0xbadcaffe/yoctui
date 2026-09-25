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
        Screen::Images => image_tab_at_column(app, area.width, column).and_then(|view| {
            (view != app.images_view).then(|| {
                let current = yoctui_model::ImagesView::ALL
                    .iter()
                    .position(|candidate| *candidate == app.images_view)
                    .unwrap_or(0) as isize;
                let destination = yoctui_model::ImagesView::ALL
                    .iter()
                    .position(|candidate| *candidate == view)
                    .unwrap_or(0) as isize;
                Action::ShiftImagesView {
                    delta: destination - current,
                }
            })
        }),
        Screen::Kernel if column < 17 => {
            Some(Action::SetKernelView(yoctui_model::PlatformView::Configuration))
        }
        Screen::Kernel if (20..36).contains(&column) => {
            Some(Action::SetKernelView(yoctui_model::PlatformView::DeviceTrees))
        }
        Screen::Firmware if column < 17 => Some(Action::SetFirmwareView(
            yoctui_model::PlatformView::Configuration,
        )),
        Screen::Firmware if (20..36).contains(&column) => Some(Action::SetFirmwareView(
            yoctui_model::PlatformView::DeviceTrees,
        )),
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

fn image_tab_at_column(
    app: &yoctui_model::App,
    width: u16,
    column: u16,
) -> Option<yoctui_model::ImagesView> {
    let mut start = 0_u16;
    for view in yoctui_model::ImagesView::ALL {
        let label = match view {
            yoctui_model::ImagesView::Artifacts => "Artifacts",
            yoctui_model::ImagesView::RootfsPackages if (78..90).contains(&width) => "Packages",
            yoctui_model::ImagesView::RootfsPackages => "Rootfs packages",
            yoctui_model::ImagesView::RootfsFilesystem => "Files",
            yoctui_model::ImagesView::SystemdServices => "systemd",
            yoctui_model::ImagesView::SystemDbus => "D-Bus",
            yoctui_model::ImagesView::UdevRules => "udev",
        };
        let label = if width < 78 && view != app.images_view {
            ""
        } else {
            label
        };
        let span = 4_u16.saturating_add(label.chars().count() as u16);
        if (start..start.saturating_add(span)).contains(&column) {
            return Some(view);
        }
        start = start.saturating_add(span).saturating_add(3);
    }
    None
}

pub(crate) fn package_row_click(
    app: &yoctui_model::App,
    area: MouseRect,
    mouse: MouseInput,
) -> Option<Action> {
    let visible = app.filtered_packages();
    let selected = visible
        .iter()
        .position(|package| app.package_selection.as_ref() == Some(&package.identity))
        .unwrap_or(0);
    let limitation_rows = matches!(
        app.package_inventory,
        yoctui_model::PackageInventoryState::Partial { .. }
    ) && area.height >= 11;
    let capacity = usize::from(area.height.saturating_sub(if limitation_rows { 8 } else { 4 }))
        .max(1);
    collection_row_delta(area, mouse, 3, capacity, visible.len(), selected)
        .map(|delta| Action::SelectPackage { delta })
}

pub(crate) fn image_artifact_row_click(
    app: &yoctui_model::App,
    area: MouseRect,
    mouse: MouseInput,
) -> Option<Action> {
    if app.images_view != yoctui_model::ImagesView::Artifacts {
        return None;
    }
    let visible = app.filtered_image_artifacts();
    let selected = visible
        .iter()
        .position(|artifact| app.image_artifact_selection.as_ref() == Some(&artifact.identity))
        .unwrap_or(0);
    let capacity = usize::from(area.height.saturating_sub(7)).max(1);
    collection_row_delta(area, mouse, 6, capacity, visible.len(), selected)
        .map(|delta| Action::SelectImageArtifact { delta })
}

fn collection_row_delta(
    area: MouseRect,
    mouse: MouseInput,
    first_row_offset: u16,
    capacity: usize,
    total: usize,
    selected: usize,
) -> Option<isize> {
    let first_row = area.y.saturating_add(first_row_offset);
    if mouse.row < first_row || mouse.row >= first_row.saturating_add(capacity as u16) {
        return None;
    }
    let viewport = yoctui_model::centered_viewport_range(Some(selected), total, capacity);
    let clicked = viewport.start + usize::from(mouse.row - first_row);
    (clicked < total).then_some(clicked as isize - selected as isize)
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
            if app.platform_menuconfig_visible() {
                leaves.push((shell, app.pane_layout.focused));
            } else {
                collect_terminal_mouse_panes(&app.pane_layout.root, shell, &mut leaves);
            }
            let single_pane = leaves.len() == 1;
            leaves
                .into_iter()
                .enumerate()
                .find_map(|(index, (area, pane))| {
                    let session_index = if single_pane {
                        app.selected_terminal_index().unwrap_or(app.pty_selection)
                    } else {
                        index
                    };
                    (session_index < app.daemon.pty_sessions.len() && area.contains(mouse))
                        .then_some(Action::SelectPtyPane {
                            pane,
                            index: session_index,
                        })
                })
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
