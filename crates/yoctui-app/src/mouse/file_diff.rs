/// Geometry mirrors file_diff_render: 3 status rows, 2 bottom rows, two halves.
pub(crate) fn file_diff_mouse_action(
    mouse: MouseInput,
    app: &yoctui_model::App,
    area: MouseRect,
) -> Option<Action> {
    use yoctui_model::FileDiffAction as Diff;
    let offline = u16::from(app.offline_notice().is_some()) * 2;
    let area = MouseRect {
        y: area.y + offline,
        height: area.height.saturating_sub(offline),
        ..area
    };
    let pane = usize::from(mouse.column >= area.x + area.width / 2);
    let first = area.y + 4;
    let viewport = usize::from(
        area.height
            .saturating_sub(if app.file_diff.comparison.is_some() {
                7
            } else {
                8
            }),
    );
    let (selection, total) = app.file_diff.comparison.as_ref().map_or_else(
        || {
            let browser = &app.file_diff.panes[pane].browser;
            (browser.selection, browser.entries.len())
        },
        |diff| (app.file_diff.selection, diff.rows.len()),
    );
    let offset = if app.file_diff.comparison.is_some() {
        app.file_diff.offset
    } else {
        app.file_diff.panes[pane].offset
    };
    let scroll = yoctui_model::BoundedScroll::new(selection, offset, viewport, total);
    let index = match mouse.kind {
        MouseKind::Down if mouse.row >= first && usize::from(mouse.row - first) < viewport => {
            scroll.offset + usize::from(mouse.row - first)
        }
        MouseKind::ScrollUp => yoctui_model::shifted_index(selection, -1, total),
        MouseKind::ScrollDown => yoctui_model::shifted_index(selection, 1, total),
        MouseKind::Down => selection,
        _ => return None,
    };
    Some(Action::FileDiff(Diff::SelectAt { pane, index }))
}
