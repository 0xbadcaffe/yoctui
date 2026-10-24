pub(crate) fn development_mouse_action(
    mouse: MouseInput,
    app: &yoctui_model::App,
    area: MouseRect,
) -> Option<Action> {
    use yoctui_model::{WorkbenchAction as W, WorkbenchFocus as F};
    let offline = u16::from(app.offline_notice().is_some()) * 2;
    let area = MouseRect {
        y: area.y + offline,
        height: area.height.saturating_sub(offline),
        ..area
    };
    if let Some(browser) = &app.workbench.folder_browser {
        let [x, y, width, height] =
            crate::workbench_folder_dialog(area.x, area.y, area.width, area.height);
        let popup = MouseRect {
            x,
            y,
            width,
            height,
        };
        if !popup.contains(mouse)
            || mouse.column <= x
            || mouse.column >= popup.right().saturating_sub(1)
            || mouse.row <= y
            || mouse.row >= popup.bottom().saturating_sub(1)
        {
            return None;
        }
        let action = match mouse.kind {
            MouseKind::ScrollUp => W::BrowseSelect(-1),
            MouseKind::ScrollDown => W::BrowseSelect(1),
            MouseKind::Down if mouse.row == y + 1 && !browser.editing_path => W::BrowseFolders,
            MouseKind::Down if mouse.row >= y + 4 && mouse.row < y + height.saturating_sub(2) => {
                let capacity = usize::from(height.saturating_sub(6));
                let first = browser.selection.saturating_sub(capacity.saturating_sub(1));
                let index = first + usize::from(mouse.row - y - 4);
                if index > browser.entries.len() {
                    return None;
                }
                if index == browser.selection && !browser.editing_path {
                    W::BrowseOpen
                } else {
                    W::BrowseSelectAt(index)
                }
            }
            _ => return None,
        };
        return Some(Action::Workbench(action));
    }
    if crate::workbench_modal_open(app) {
        return None;
    }
    let Some(session) = app.workbench.current() else {
        return match mouse.kind {
            MouseKind::ScrollUp => Some(Action::Workbench(W::Select(-1))),
            MouseKind::ScrollDown => Some(Action::Workbench(W::Select(1))),
            MouseKind::Down
                if mouse.row >= area.y + 4 && mouse.row < area.bottom().saturating_sub(1) =>
            {
                let capacity = usize::from(area.height.saturating_sub(6)).max(1);
                let first = app
                    .workbench
                    .selection
                    .saturating_sub(capacity.saturating_sub(1));
                let index = first + usize::from(mouse.row - area.y - 4);
                Some(Action::Workbench(
                    if index == app.workbench.selection && app.focus == FocusTarget::Workspace {
                        W::Open
                    } else {
                        W::SelectAt(index)
                    },
                ))
            }
            _ => None,
        };
    };
    let layout = crate::development_workbench_layout(
        area.x,
        area.y,
        area.width,
        area.height,
        session.panel_zoom,
    );
    let rect = |a: [u16; 4]| MouseRect {
        x: a[0],
        y: a[1],
        width: a[2],
        height: a[3],
    };
    let tree = rect(layout.explorer);
    if tree.contains(mouse) {
        let folder = session.folders.get(session.folder)?;
        if matches!(mouse.kind, MouseKind::ScrollUp | MouseKind::ScrollDown) {
            return Some(Action::Workbench(W::Select(
                if mouse.kind == MouseKind::ScrollUp {
                    -1
                } else {
                    1
                },
            )));
        }
        if mouse.kind != MouseKind::Down || mouse.row <= tree.y {
            return None;
        }
        let available = usize::from(tree.height.saturating_sub(2))
            .saturating_sub(session.folders.len() + usize::from(folder.error.is_some()));
        let first = folder
            .browser
            .selection
            .saturating_sub(available.saturating_sub(1));
        let mut row = usize::from(mouse.row - tree.y - 1);
        for index in 0..session.folders.len() {
            if row == 0 {
                return Some(Action::Workbench(W::FolderAt(index)));
            }
            row -= 1;
            if index == session.folder {
                let error_rows = usize::from(folder.error.is_some());
                let count = folder
                    .browser
                    .entries
                    .len()
                    .saturating_sub(first)
                    .min(available);
                if row < error_rows {
                    return None;
                }
                if row - error_rows < count {
                    let selected = first + row - error_rows;
                    return Some(Action::Workbench(
                        if selected == folder.browser.selection && session.focus == F::Files {
                            W::Open
                        } else {
                            W::SelectAt(selected)
                        },
                    ));
                }
                row = row.saturating_sub(error_rows + count);
            }
        }
        return None;
    }
    if rect(layout.tabs).contains(mouse) && mouse.kind == MouseKind::Down {
        let count = usize::from(layout.tabs[2] / 20).max(1);
        let first = session.tab.saturating_sub(count - 1);
        return Some(Action::Workbench(W::SelectTab(
            first + usize::from(mouse.column - layout.tabs[0]) / 20,
        )));
    }
    if rect(layout.editor).contains(mouse) {
        if matches!(mouse.kind, MouseKind::Down | MouseKind::Drag)
            && mouse.column == layout.editor[0] + layout.editor[2].saturating_sub(1)
        {
            let tab = session.tabs.get(session.tab)?;
            let rows = usize::from(layout.editor[3].saturating_sub(2)).max(1);
            let track = usize::from(mouse.row.saturating_sub(layout.editor[1] + 1)).min(rows - 1);
            let target = track * tab.document.line_count().saturating_sub(rows)
                / rows.saturating_sub(1).max(1);
            return Some(Action::Workbench(W::ScrollEditor(
                target as isize - tab.viewport.top as isize,
            )));
        }
        if matches!(mouse.kind, MouseKind::ScrollUp | MouseKind::ScrollDown) {
            return Some(Action::Workbench(W::ScrollEditor(
                if mouse.kind == MouseKind::ScrollUp {
                    -3
                } else {
                    3
                },
            )));
        }
        if mouse.kind == MouseKind::Down
            && mouse.column > layout.editor[0]
            && mouse.column < layout.editor[0] + layout.editor[2].saturating_sub(1)
            && mouse.row > layout.editor[1]
            && mouse.row < layout.editor[1] + layout.editor[3].saturating_sub(1)
        {
            let tab = session.tabs.get(session.tab)?;
            let first = tab.viewport.top;
            let line = first + usize::from(mouse.row - layout.editor[1] - 1);
            let gutter = yoctui_model::workbench_gutter_width(tab.document.line_count());
            if mouse.column < layout.editor[0] + 1 + gutter as u16 {
                return Some(Action::Workbench(W::BreakpointAt(line + 1)));
            }
            let visual = tab.viewport.left
                + usize::from(
                    mouse
                        .column
                        .saturating_sub(layout.editor[0] + 1 + gutter as u16),
                );
            let text = tab.document.text.split('\n').nth(line).unwrap_or_default();
            let cursor = tab.document.position();
            let visual = visual.saturating_sub(usize::from(
                line == cursor.line
                    && visual > yoctui_model::workbench_visual_column(text, cursor.column),
            ));
            return Some(Action::Workbench(W::Edit(
                PopupEditorCommand::SelectPosition {
                    line,
                    column: yoctui_model::workbench_source_column(text, visual),
                    extend: false,
                },
            )));
        }
    }
    if rect(layout.output).contains(mouse) {
        if session.focus == F::Terminal {
            return match mouse.kind {
                MouseKind::ScrollUp => Some(Action::TerminalScroll { delta: 3 }),
                MouseKind::ScrollDown => Some(Action::TerminalScroll { delta: -3 }),
                _ => None,
            };
        }
        if mouse.kind == MouseKind::Down {
            return Some(Action::Workbench(W::Focus(F::Output)));
        }
    }
    None
}
