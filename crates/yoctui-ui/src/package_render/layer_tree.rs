pub(crate) struct LayerTreeWidgetProjection {
    pub(crate) items: Vec<TreeItem<'static, PathBuf>>,
    pub(crate) selected: Option<Vec<PathBuf>>,
    pub(crate) opened: Vec<Vec<PathBuf>>,
}

#[derive(Default)]
pub(crate) struct LayerTreeWidgetState {
    pub(crate) selected: Option<Vec<PathBuf>>,
    pub(crate) opened: Vec<Vec<PathBuf>>,
}

pub(crate) fn layer_tree_entry_label(
    browser: &LayerBrowser,
    entry: &LayerBrowserEntry,
    unicode: bool,
    widget_branch: bool,
    leading_depth: usize,
) -> String {
    let name = entry.path.file_name().map_or_else(
        || entry.path.display().to_string(),
        |name| name.to_string_lossy().into_owned(),
    );
    let marker = if widget_branch {
        ""
    } else if entry.is_dir {
        match (unicode, browser.expanded.contains(&entry.path)) {
            (true, true) => "▾ ",
            (true, false) => "▸ ",
            (false, true) => "- ",
            (false, false) => "+ ",
        }
    } else {
        "  "
    };
    let git = match entry.git {
        GitFileState::Modified => " M",
        GitFileState::Untracked => " ?",
        GitFileState::Ignored => " I",
        GitFileState::Clean => "  ",
        GitFileState::Unavailable => " -",
    };
    format!(
        "{}{marker}{name}{}{git}",
        "  ".repeat(leading_depth),
        if entry.is_dir { "/" } else { "" }
    )
}

pub(crate) fn nested_layer_tree_items(
    browser: &LayerBrowser,
    entries: &[(usize, &LayerBrowserEntry)],
    cursor: &mut usize,
    depth: usize,
    parent_ids: &[PathBuf],
    state: &mut LayerTreeWidgetState,
    unicode: bool,
) -> std::io::Result<Vec<TreeItem<'static, PathBuf>>> {
    let mut items = Vec::new();
    while let Some((index, entry)) = entries.get(*cursor).copied() {
        if entry.depth < depth {
            break;
        }
        if entry.depth > depth {
            // LayerBrowser's bounded DFS normally advances one level at a
            // time. Treat a malformed gap as a visible leaf rather than
            // allowing a renderer-only projection to panic.
            let label = layer_tree_entry_label(
                browser,
                entry,
                unicode,
                false,
                entry.depth.saturating_sub(depth),
            );
            let mut ids = parent_ids.to_vec();
            ids.push(entry.path.clone());
            if index == browser.selection {
                state.selected = Some(ids);
            }
            items.push(TreeItem::new_leaf(entry.path.clone(), label));
            *cursor += 1;
            continue;
        }

        *cursor += 1;
        let mut ids = parent_ids.to_vec();
        ids.push(entry.path.clone());
        let has_nested_children = entry.is_dir
            && entries
                .get(*cursor)
                .is_some_and(|(_, next)| next.depth > depth);
        let children = if has_nested_children {
            nested_layer_tree_items(browser, entries, cursor, depth + 1, &ids, state, unicode)?
        } else {
            Vec::new()
        };
        if !children.is_empty() {
            state.opened.push(ids.clone());
        }
        if index == browser.selection {
            state.selected = Some(ids);
        }
        let label = layer_tree_entry_label(browser, entry, unicode, !children.is_empty(), 0);
        let item = if children.is_empty() {
            TreeItem::new_leaf(entry.path.clone(), label)
        } else {
            TreeItem::new(entry.path.clone(), label, children)?
        };
        items.push(item);
    }
    Ok(items)
}

pub(crate) fn layer_tree_widget_projection(
    browser: &LayerBrowser,
    entries: &[(usize, &LayerBrowserEntry)],
    unicode: bool,
    filtered: bool,
) -> std::io::Result<LayerTreeWidgetProjection> {
    if filtered {
        let mut selected = None;
        let items = entries
            .iter()
            .map(|(index, entry)| {
                let id = entry.path.clone();
                if *index == browser.selection {
                    selected = Some(vec![id.clone()]);
                }
                TreeItem::new_leaf(
                    id,
                    layer_tree_entry_label(browser, entry, unicode, false, entry.depth),
                )
            })
            .collect();
        return Ok(LayerTreeWidgetProjection {
            items,
            selected,
            opened: Vec::new(),
        });
    }

    let mut cursor = 0;
    let mut state = LayerTreeWidgetState::default();
    let items =
        nested_layer_tree_items(browser, entries, &mut cursor, 0, &[], &mut state, unicode)?;
    Ok(LayerTreeWidgetProjection {
        items,
        selected: state.selected,
        opened: state.opened,
    })
}

pub(crate) fn layer_browser_left_width(browser: &LayerBrowser, total_width: u16) -> u16 {
    let configured = browser.layer.chars().count().saturating_add(18);
    let tree = browser
        .entries
        .iter()
        .map(|entry| {
            entry
                .path
                .file_name()
                .map_or(0, |name| name.to_string_lossy().chars().count())
                .saturating_add(entry.depth.saturating_mul(2))
                .saturating_add(8)
        })
        .max()
        .unwrap_or(0);
    let useful = configured.max(tree).clamp(38, 54) as u16;
    let ratio_cap = total_width.saturating_mul(42) / 100;
    let preview_floor = if total_width >= 100 { 58 } else { 32 };
    let preview_cap = total_width.saturating_sub(preview_floor);
    useful.min(ratio_cap.max(1)).min(preview_cap.max(1))
}
