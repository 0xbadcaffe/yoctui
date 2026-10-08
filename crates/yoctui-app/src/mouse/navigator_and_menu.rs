pub(crate) fn literal_navigator_selection_at_row(
    app: &yoctui_model::App,
    row: usize,
) -> Option<usize> {
    use yoctui_model::WorkspaceDestination as Destination;

    let layer_rows = app.workspace.layers.len().clamp(1, 7);
    let recipe_rows = app.workspace.recipes.len().clamp(1, 3);
    let image_rows = app.available_images.len().clamp(1, 2);
    let layers_end = 1 + layer_rows;
    if row < layers_end {
        return App::navigator_selection_for_destination(Destination::Layers);
    }
    let recipes_start = layers_end;
    let recipes_end = recipes_start + 1 + recipe_rows;
    if row < recipes_end {
        return App::navigator_selection_for_destination(Destination::Recipes);
    }
    let images_start = recipes_end;
    let images_end = images_start + 1 + image_rows;
    if row < images_end {
        return App::navigator_selection_for_destination(Destination::Images);
    }
    let tasks_start = images_end;
    if row == tasks_start {
        return App::navigator_selection_for_destination(Destination::Tasks);
    }
    const TASK_DESTINATIONS: [Destination; 8] = [
        Destination::Tasks,
        Destination::Testing,
        Destination::Qa,
        Destination::Devtool,
        Destination::QemuWic,
        Destination::Sdk,
        Destination::Security,
        Destination::Maintenance,
    ];
    TASK_DESTINATIONS
        .get(row - tasks_start - 1)
        .copied()
        .and_then(App::navigator_selection_for_destination)
}

/// Centered application-menu bounds shared by rendering and mouse hit testing.
pub fn application_menu_bounds(
    _app: &App,
    width: u16,
    height: u16,
    _items: usize,
) -> Option<(u16, u16, u16, u16)> {
    if width < 64 || height < 16 {
        return None;
    }
    let menu_width = 100.min(width.saturating_sub(4));
    let menu_height = 28.min(height.saturating_sub(6));
    Some((
        width.saturating_sub(menu_width) / 2,
        height.saturating_sub(menu_height) / 2,
        menu_width,
        menu_height,
    ))
}
fn application_menu_mouse_action(
    mouse: MouseInput,
    app: &App,
    width: u16,
    height: u16,
) -> Option<Action> {
    if app.menu.kind != Some(yoctui_model::MenuKind::Application) {
        return None;
    }
    let items = app.active_menu_items();
    let (left, top, width, height) = application_menu_bounds(app, width, height, items.len())?;
    if mouse.column <= left
        || mouse.column >= left + width - 1
        || mouse.row <= top
        || mouse.row >= top + height - 1
    {
        return None;
    }
    match mouse.kind {
        MouseKind::ScrollUp => return Some(Action::SelectMenuItem { delta: -1 }),
        MouseKind::ScrollDown => return Some(Action::SelectMenuItem { delta: 1 }),
        MouseKind::Down => {}
        _ => return None,
    }
    if mouse.row == top + 1 {
        let mut column = left + 1;
        for (index, group) in yoctui_model::ApplicationMenuGroup::ALL.iter().enumerate() {
            let end = column + group.label().len() as u16 + 2;
            if mouse.column < end {
                return Some(Action::SelectMenuGroup {
                    delta: index as isize - app.menu.group_selection as isize,
                });
            }
            column = end;
        }
        return None;
    }
    if mouse.row >= top + 3 && mouse.row < top + height - 2 {
        let selected = app.menu.item_selection.min(items.len().saturating_sub(1));
        let range = yoctui_model::centered_viewport_range(
            (!items.is_empty()).then_some(selected),
            items.len(),
            usize::from(height.saturating_sub(5)).max(1),
        );
        let index = range.start + usize::from(mouse.row - top - 3);
        if index < range.end {
            return Some(Action::SelectMenuItem {
                delta: index as isize - selected as isize,
            });
        }
    }
    None
}
