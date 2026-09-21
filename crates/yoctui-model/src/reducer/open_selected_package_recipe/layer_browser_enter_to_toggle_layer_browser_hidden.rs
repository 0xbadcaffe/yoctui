use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::LayerBrowserEnter => {
            let selected = app
                .layer_browser
                .as_ref()
                .and_then(LayerBrowser::selected_entry)
                .cloned();
            if let Some(entry) = selected.filter(|entry| entry.is_dir) {
                let browser = app.layer_browser.as_mut().expect("browser was selected");
                if browser.expanded.remove(&entry.path) {
                    browser.rebuild(Some(&entry.path));
                    return None;
                }
                if browser.nodes.contains_key(&entry.path) {
                    browser.expanded.insert(entry.path.clone());
                    browser.rebuild(Some(&entry.path));
                    return None;
                }
                return Some(Effect::LoadLayerBrowserDirectory {
                    layer: browser.layer.clone(),
                    root: browser.root.clone(),
                    directory: entry.path,
                });
            }
            return update(app, Action::EditSelectedLayerBrowserFile);
        }
        Action::LayerBrowserUp => {
            if let Some(browser) = app.layer_browser.as_mut()
                && let Some(entry) = browser.selected_entry().cloned()
            {
                if entry.is_dir && browser.expanded.remove(&entry.path) {
                    browser.rebuild(Some(&entry.path));
                } else if let Some(parent) = entry.path.parent()
                    && parent != browser.root
                    && let Some(index) = browser
                        .entries
                        .iter()
                        .position(|candidate| candidate.path == parent)
                {
                    browser.selection = index;
                }
            }
        }
        Action::CloseLayerBrowser => app.layer_browser = None,
        Action::RefreshLayerBrowser => {
            if let Some(browser) = app.layer_browser.as_ref() {
                let directory = browser
                    .selected_entry()
                    .map(|entry| {
                        if entry.is_dir {
                            entry.path.clone()
                        } else {
                            entry.path.parent().unwrap_or(&browser.root).to_path_buf()
                        }
                    })
                    .unwrap_or_else(|| browser.root.clone());
                return Some(Effect::LoadLayerBrowserDirectory {
                    layer: browser.layer.clone(),
                    root: browser.root.clone(),
                    directory,
                });
            }
        }
        Action::ToggleLayerBrowserHidden => {
            if let Some(browser) = app.layer_browser.as_mut() {
                let selected = browser.selected_entry().map(|entry| entry.path.clone());
                browser.show_hidden = !browser.show_hidden;
                browser.rebuild(selected.as_ref());
            }
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
