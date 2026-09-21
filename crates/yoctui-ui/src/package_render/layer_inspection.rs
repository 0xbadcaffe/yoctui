pub(crate) fn layer_entry_metadata(
    app: &App,
    browser: &LayerBrowser,
    entry: &LayerBrowserEntry,
) -> String {
    let display_path = if entry.path.is_absolute() {
        entry.path.clone()
    } else {
        browser.root.join(&entry.path)
    };
    let relationship = layer_relationship(app, &browser.layer);
    let size = if entry.is_dir {
        "directory".into()
    } else {
        entry
            .size
            .map_or_else(|| "unavailable".into(), |size| format!("{size} bytes"))
    };
    let modified = entry
        .modified
        .map_or_else(|| "unavailable".into(), timestamp_text);
    let compatibility = relationship.map_or_else(
        || "unavailable".into(),
        |value| {
            if value.compatible.is_empty() {
                "not reported".into()
            } else {
                value.compatible.join(", ")
            }
        },
    );
    format!(
        "Path: {}\nType/size: {size}\nModified: {modified}\nGit: {}\nLayer: {}\nCompatibility: {compatibility}",
        display_path.display(),
        git_state_text(entry.git),
        browser.layer
    )
}

pub(crate) fn layer_inspector_text(app: &App, browser: &LayerBrowser) -> Text<'static> {
    let Some(entry) = browser.selected_entry() else {
        return Text::from("This layer is empty.");
    };
    let metadata = layer_entry_metadata(app, browser, entry);
    let relationship = layer_relationship(app, &browser.layer);
    match browser.inspector_mode {
        LayerInspectorMode::Git => Text::from(format!(
            "{metadata}\n\nGit state: {}\nGit status is detected per loaded subtree; missing Git is non-fatal.",
            git_state_text(entry.git)
        )),
        LayerInspectorMode::Metadata => Text::from(metadata),
        LayerInspectorMode::Dependencies => Text::from(format!(
            "{metadata}\n\nDepends: {}\nOverlays: {}\nAppends: {}",
            relationship.map_or("unavailable".into(), |value| {
                if value.depends.is_empty() {
                    "none reported".into()
                } else {
                    value.depends.join(", ")
                }
            }),
            relationship.map_or("unavailable".into(), |value| {
                if value.overlays.is_empty() {
                    "none reported".into()
                } else {
                    value.overlays.join(", ")
                }
            }),
            relationship.map_or("unavailable".into(), |value| {
                if value.appends.is_empty() {
                    "none reported".into()
                } else {
                    value.appends.join(", ")
                }
            })
        )),
        LayerInspectorMode::Preview if entry.is_dir => Text::from(format!(
            "{metadata}\n\nDirectory contents are loaded only when expanded."
        )),
        LayerInspectorMode::Preview => match browser.preview_kind {
            PreviewKind::Binary => Text::from(format!(
                "{metadata}\n\nBinary preview unavailable.{}",
                if browser.preview_truncated {
                    "\nPreview exceeds the 64 KiB bound."
                } else {
                    ""
                }
            )),
            PreviewKind::Unavailable => Text::from(format!(
                "{metadata}\n\nPreview unavailable or still loading."
            )),
            PreviewKind::Text => {
                let file_name = entry
                    .path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("");
                let mut preview = numbered_source_preview(&browser.preview, file_name, app);
                if browser.preview_truncated {
                    preview.lines.insert(
                        0,
                        Line::from("[preview truncated at 64 KiB]").style(
                            ThemePalette::for_app(app)
                                .role(ThemePalette::for_app(app).warning, Modifier::BOLD),
                        ),
                    );
                }
                preview
            }
        },
    }
}
