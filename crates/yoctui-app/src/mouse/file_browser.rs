fn file_browser_is_visible(app: &yoctui_model::App) -> bool {
    (app.screen == Screen::Layers && app.layer_browser.is_some()) || app.rootfs_browser_active()
}

fn file_browser_row_click(
    mouse: MouseInput,
    app: &yoctui_model::App,
    region: MouseRect,
) -> Option<Action> {
    if !file_browser_is_visible(app) {
        return None;
    }
    let browser = app.layer_browser.as_ref()?;
    let offline = u16::from(app.offline_notice().is_some()) * 2;
    let rootfs = u16::from(browser.is_rootfs());
    let geometry = crate::layer_browser_layout(browser, app.workspace.layers.len(), [
        region.x.saturating_add(rootfs),
        region.y.saturating_add(offline + rootfs * 2),
        region.width.saturating_sub(rootfs * 2),
        region.height.saturating_sub(offline + rootfs * 3),
    ]);
    let [x, y, width, height] = geometry.tree;
    let rows = MouseRect {
        x: x + 1,
        y: y + 1,
        width: width.saturating_sub(2),
        height: height.saturating_sub(2),
    };
    if !rows.contains(mouse) {
        return None;
    }
    let query = app.metadata_query.to_ascii_lowercase();
    let matched = browser.matching_entries(&query).map(|(index, _)| index).collect::<Vec<_>>();
    let selection = matched.iter().position(|index| *index == browser.selection).unwrap_or(0);
    let viewport = yoctui_model::file_browser_scroll(selection, usize::from(rows.height), matched.len()).visible_range();
    let index = *matched.get(viewport).unwrap_or_default().get(usize::from(mouse.row - rows.y))?;
    Some(Action::SelectLayerBrowserEntryAt { index })
}
