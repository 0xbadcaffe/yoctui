pub(crate) fn development_mouse_action(
    mouse: MouseInput,
    app: &yoctui_model::App,
    area: MouseRect,
) -> Option<Action> {
    use yoctui_model::{WorkbenchAction as W, WorkbenchFocus as F};
    if app.workbench.form.is_some() || app.workbench.menu.is_some() {
        return None;
    }
    let offline = u16::from(app.offline_notice().is_some()) * 2;
    let area = MouseRect {
        y: area.y + offline,
        height: area.height.saturating_sub(offline),
        ..area
    };
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
        if matches!(mouse.kind, MouseKind::ScrollUp | MouseKind::ScrollDown) {
            return Some(Action::Workbench(W::Edit(
                if mouse.kind == MouseKind::ScrollUp {
                    PopupEditorCommand::Up
                } else {
                    PopupEditorCommand::Down
                },
            )));
        }
        if mouse.kind == MouseKind::Down
            && mouse.row > layout.editor[1]
            && mouse.row < layout.editor[1] + layout.editor[3].saturating_sub(1)
        {
            let tab = session.tabs.get(session.tab)?;
            let position = tab.document.position();
            let first = position.line.saturating_sub(
                usize::from(layout.editor[3].saturating_sub(2))
                    .max(1)
                    .saturating_sub(2),
            );
            let horizontal = position
                .column
                .saturating_sub(usize::from(layout.editor[2].saturating_sub(5)));
            return Some(Action::Workbench(W::Edit(
                PopupEditorCommand::SelectPosition {
                    line: first + usize::from(mouse.row - layout.editor[1] - 1),
                    column: horizontal
                        + usize::from(mouse.column.saturating_sub(layout.editor[0] + 1)),
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
