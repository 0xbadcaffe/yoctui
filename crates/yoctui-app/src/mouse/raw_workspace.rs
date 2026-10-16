pub(crate) fn raw_mouse_action(mouse: MouseInput, app: &yoctui_model::App,
    width: u16, height: u16) -> Option<yoctui_model::RawModeAction> {
    use yoctui_model::{RawBrowserColumn, RawModeAction as Raw, RawModeView as View};
    let state = &app.raw_mode;
    if state.modal_active() || matches!(state.view, View::Form | View::Preview) { return None; }
    let region = workbench_mouse_region(mouse, app, workbench_shell(app, width, height)?)?;
    if region.target != FocusTarget::Workspace { return None; }
    let mut area = region.area;
    let status = state.workspace_status_rows().min(area.height);
    area.y += status;
    area.height -= status;
    if !area.contains(mouse) { return None; }
    let category_width = yoctui_model::raw_category_column_width(area.width);
    let column = if width >= 100 {
        if mouse.column < area.x + category_width { RawBrowserColumn::Categories } else { RawBrowserColumn::Commands }
    } else { state.browser_column };
    let delta = match mouse.kind { MouseKind::ScrollUp => Some(-1), MouseKind::ScrollDown => Some(1), _ => None };
    if let Some(delta) = delta {
        return Some(match state.view {
            View::Execution => Raw::ScrollOutput { vertical: -delta, horizontal: 0 },
            View::History => Raw::SelectHistory { delta },
            View::Favorites => Raw::SelectFavorite { delta },
            _ if column == RawBrowserColumn::Categories => Raw::SelectCategory { delta },
            _ => Raw::SelectCommand { delta },
        });
    }
    if mouse.kind != MouseKind::Down || mouse.row <= area.y || mouse.row + 1 >= area.bottom() { return None; }
    let row = usize::from(mouse.row - area.y - 1);
    match state.view {
        View::Favorites => {
            let visible = (usize::from(area.height.saturating_sub(3)) / 4).max(1);
            let range = yoctui_model::centered_viewport_range(Some(state.favorite_selection), state.favorites.len(), visible);
            let target = range.start + row / 4;
            (target < range.end).then_some(Raw::SelectFavorite { delta: target as isize - state.favorite_selection as isize })
        }
        View::History => {
            let detail_height = if area.height >= 18 { 7 } else { 3 };
            let list_height = area.height.saturating_sub(detail_height);
            if mouse.row + 1 >= area.y + list_height { return None; }
            let count = state.history_request_ids().len();
            let range = yoctui_model::centered_viewport_range(Some(state.history_selection), count,
                (usize::from(list_height.saturating_sub(2)) / 2).max(1));
            let target = range.start + row / 2;
            (target < range.end).then_some(Raw::SelectHistory { delta: target as isize - state.history_selection as isize })
        }
        View::Browser if column == RawBrowserColumn::Categories => {
            let categories = yoctui_model::builtin_raw_catalog().browser_categories();
            let current = categories.iter().position(|category| Some(&category.id) == state.category.as_ref()).unwrap_or(0);
            let range = yoctui_model::centered_viewport_range(Some(current), categories.len(), usize::from(area.height.saturating_sub(2)));
            let target = range.start + row;
            (target < range.end).then_some(Raw::SelectCategory { delta: target as isize - current as isize })
        }
        View::Browser => {
            let commands = state.visible_commands(yoctui_model::builtin_raw_catalog());
            let current = commands.iter().position(|command| Some(&command.id) == state.command.as_ref()).unwrap_or(0);
            let range = yoctui_model::centered_viewport_range(Some(current), commands.len(),
                (usize::from(area.height.saturating_sub(2)) / 2).max(1));
            let target = range.start + row / 2;
            (target < range.end).then_some(Raw::SelectCommand { delta: target as isize - current as isize })
        }
        _ => None,
    }
}
