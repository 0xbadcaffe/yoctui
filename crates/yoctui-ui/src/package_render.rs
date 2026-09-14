//! Package render.
use super::*;

pub(crate) fn packages_workspace(frame: &mut Frame, app: &App, area: Rect) {
    let palette = ThemePalette::for_app(app);
    let visible = app.filtered_packages();
    let selected = visible
        .iter()
        .position(|package| app.package_selection.as_ref() == Some(&package.identity));
    let workspace = Layout::vertical([Constraint::Length(1), Constraint::Min(3)]).split(area);
    frame.render_widget(
        Paragraph::new(search_line(
            app,
            &app.package_query,
            app.package_searching,
            selected,
            visible.len(),
            SearchNavigation::Results,
            SearchExit::Done,
            workspace[0].width,
        )),
        workspace[0],
    );
    let area = workspace[1];
    let block = pane_block(app, "Packages", app.focus == FocusTarget::Workspace);
    match &app.package_inventory {
        PackageInventoryState::NotLoaded => frame.render_widget(
            Paragraph::new(
                "Package data has not been loaded.\n\nEnter this workspace or press R to query generated pkgdata.",
            )
            .block(block)
            .wrap(Wrap { trim: false }),
            area,
        ),
        PackageInventoryState::Loading { .. } => frame.render_widget(
            Paragraph::new(
                "Loading authoritative package inventory…\n\nThe workspace remains responsive. Press c to cancel.",
            )
            .block(block)
            .wrap(Wrap { trim: false }),
            area,
        ),
        PackageInventoryState::AvailableEmpty { .. } => frame.render_widget(
            Paragraph::new("No built runtime packages were reported by oe-pkgdata-util.")
                .block(block)
                .wrap(Wrap { trim: false }),
            area,
        ),
        PackageInventoryState::Failed { message, .. } => frame.render_widget(
            Paragraph::new(format!(
                "Package inventory failed.\n\n{message}\n\nIf generated pkgdata is missing, build a target through do_package and press R."
            ))
            .block(block)
            .wrap(Wrap { trim: false }),
            area,
        ),
        PackageInventoryState::Available { .. } | PackageInventoryState::Partial { .. } => {
            let limitations = match &app.package_inventory {
                PackageInventoryState::Partial { limitations, .. } => limitations.as_slice(),
                _ => &[],
            };
            let rows_area = if limitations.is_empty() || area.height < 10 {
                area
            } else {
                Layout::vertical([Constraint::Min(4), Constraint::Length(4)]).split(area)[0]
            };
            let capacity = usize::from(rows_area.height.saturating_sub(3)).max(1);
            let viewport = yoctui_model::centered_viewport_range(
                selected,
                visible.len(),
                capacity,
            );
            let rows = visible[viewport]
                .iter()
                .copied()
                .map(|package| {
                    let selected = app.package_selection.as_ref() == Some(&package.identity);
                    let style = if selected {
                        palette.selected()
                    } else {
                        palette.base()
                    };
                    let name = package.identity.name.clone();
                    let recipe = package_field_text(&package.recipe);
                    let version = package_field_text(&package.version);
                    let size = package
                        .installed_size_bytes
                        .available()
                        .map_or_else(|| "unavailable".into(), |value| format_bytes(*value));
                    let license = package_field_text(&package.license);
                    Row::new(vec![name, recipe, version, size, license]).style(style)
                })
                .collect::<Vec<_>>();
            let widths = if area.width >= 68 {
                vec![
                    Constraint::Percentage(27),
                    Constraint::Percentage(22),
                    Constraint::Percentage(18),
                    Constraint::Percentage(14),
                    Constraint::Percentage(19),
                ]
            } else if area.width >= 44 {
                vec![
                    Constraint::Percentage(42),
                    Constraint::Percentage(33),
                    Constraint::Percentage(25),
                    Constraint::Length(0),
                    Constraint::Length(0),
                ]
            } else {
                vec![
                    Constraint::Percentage(100),
                    Constraint::Length(0),
                    Constraint::Length(0),
                    Constraint::Length(0),
                    Constraint::Length(0),
                ]
            };
            let header = Row::new(vec!["Package", "Recipe", "Version", "Size", "License"])
                .style(palette.role(palette.accent, Modifier::BOLD));
            frame.render_widget(
                Table::new(rows, widths)
                    .header(header)
                    .block(block)
                    .column_spacing(1),
                rows_area,
            );
            if !limitations.is_empty() && area.height >= 10 {
                let split =
                    Layout::vertical([Constraint::Min(4), Constraint::Length(4)]).split(area);
                frame.render_widget(
                    Paragraph::new(
                        limitations
                            .iter()
                            .take(2)
                            .map(|value| format!("! {value}"))
                            .collect::<Vec<_>>()
                            .join("\n"),
                    )
                    .style(palette.role(palette.warning, Modifier::BOLD))
                    .block(Block::default().title("Partial result").borders(Borders::ALL))
                    .wrap(Wrap { trim: true }),
                    split[1],
                );
            }
        }
    }
}

pub(crate) fn package_field_text(field: &PackageField<String>) -> String {
    match field {
        PackageField::Available(value) if value.is_empty() => "empty".into(),
        PackageField::Available(value) => value.clone(),
        PackageField::Unavailable => "unavailable".into(),
    }
}

pub(crate) fn package_path_field_text(field: &PackageField<std::path::PathBuf>) -> String {
    match field {
        PackageField::Available(value) => value.display().to_string(),
        PackageField::Unavailable => "unavailable".into(),
    }
}

pub(crate) fn package_identity_list(
    field: &PackageField<Vec<PackageIdentity>>,
    selected: Option<&PackageIdentity>,
) -> Vec<String> {
    match field {
        PackageField::Unavailable => vec!["  unavailable".into()],
        PackageField::Available(values) if values.is_empty() => vec!["  empty".into()],
        PackageField::Available(values) => values
            .iter()
            .map(|identity| {
                format!(
                    "{} {}",
                    if selected == Some(identity) {
                        "▶"
                    } else {
                        " "
                    },
                    identity.name
                )
            })
            .collect(),
    }
}

pub(crate) fn package_path_list(field: &PackageField<Vec<std::path::PathBuf>>) -> Vec<String> {
    match field {
        PackageField::Unavailable => vec!["  unavailable".into()],
        PackageField::Available(values) if values.is_empty() => vec!["  empty".into()],
        PackageField::Available(values) => values
            .iter()
            .take(64)
            .map(|path| format!("  {}", path.display()))
            .collect(),
    }
}

pub(crate) fn package_membership_text(field: &PackageField<Vec<String>>) -> String {
    match field {
        PackageField::Unavailable => "unavailable".into(),
        PackageField::Available(values) if values.is_empty() => "empty".into(),
        PackageField::Available(values) => values.join(", "),
    }
}

pub(crate) fn package_inspector_text(app: &App) -> String {
    let Some(package) = app.selected_package() else {
        return match &app.package_inventory {
            PackageInventoryState::Loading { .. } => {
                "Package inventory is loading.\n\nNo package is selected yet.".into()
            }
            PackageInventoryState::Failed { message, .. } => {
                format!("Package inventory failed.\n\n{message}")
            }
            PackageInventoryState::AvailableEmpty { .. } => {
                "The authoritative package inventory is empty.".into()
            }
            _ => "Select a package to inspect its typed metadata.".into(),
        };
    };
    let mut lines = vec![
        format!("Package: {}", package.identity.name),
        format!("Recipe: {}", package_field_text(&package.recipe)),
        format!("Provider: {}", package_path_field_text(&package.provider)),
        format!("Version: {}", package_field_text(&package.version)),
        format!(
            "Installed size: {}",
            package
                .installed_size_bytes
                .available()
                .map_or_else(|| "unavailable".into(), |value| format_bytes(*value))
        ),
        format!("License: {}", package_field_text(&package.license)),
        format!(
            "Image membership: {}",
            package_membership_text(&package.image_membership)
        ),
        String::new(),
    ];
    match app.selected_package_detail() {
        None | Some(PackageDetailState::NotLoaded) => {
            lines.push("Detail: not loaded (press Enter)".into());
        }
        Some(PackageDetailState::Loading { .. }) => {
            lines.push("Detail: loading… (press c to cancel)".into());
        }
        Some(PackageDetailState::Failed { message, .. }) => {
            lines.push(format!("Detail: failed\n{message}"));
        }
        Some(PackageDetailState::AvailableEmpty { .. }) => {
            lines.push("Detail: available-empty".into());
            lines.push("Files: empty".into());
            lines.push("Runtime dependencies: empty".into());
            lines.push("Reverse dependencies: empty".into());
        }
        Some(
            PackageDetailState::Available { detail, .. }
            | PackageDetailState::Partial { detail, .. },
        ) => {
            lines.push("Files:".into());
            lines.extend(package_path_list(&detail.files));
            lines.push(String::new());
            lines.push(format!(
                "{} runtime dependencies:",
                if app.package_dependency_reverse {
                    " "
                } else {
                    "▶"
                }
            ));
            lines.extend(package_identity_list(
                &detail.runtime_dependencies,
                (!app.package_dependency_reverse)
                    .then(|| app.selected_package_dependency())
                    .flatten(),
            ));
            lines.push(String::new());
            lines.push(format!(
                "{} reverse dependencies:",
                if app.package_dependency_reverse {
                    "▶"
                } else {
                    " "
                }
            ));
            lines.extend(package_identity_list(
                &detail.reverse_dependencies,
                app.package_dependency_reverse
                    .then(|| app.selected_package_dependency())
                    .flatten(),
            ));
            if let Some(PackageDetailState::Partial { limitations, .. }) =
                app.selected_package_detail()
            {
                lines.push(String::new());
                lines.push("Partial detail:".into());
                lines.extend(limitations.iter().map(|value| format!("! {value}")));
            }
        }
    }
    if !app.package_navigation.is_empty() {
        lines.push(String::new());
        lines.push(format!(
            "Navigation history: {} item(s); press u to return",
            app.package_navigation.len()
        ));
    }
    lines.join("\n")
}

pub(crate) fn recipes(frame: &mut Frame, app: &App, area: Rect) {
    let recipes = app
        .workspace
        .recipes
        .iter()
        .enumerate()
        .filter(|(_, recipe)| {
            matches_metadata(
                &app.metadata_query,
                &[
                    recipe.name.as_str(),
                    recipe.version.as_deref().unwrap_or(""),
                    recipe.preferred_version.as_deref().unwrap_or(""),
                    recipe.layer.as_deref().unwrap_or(""),
                    recipe
                        .file
                        .as_ref()
                        .and_then(|path| path.to_str())
                        .unwrap_or(""),
                ],
            )
        })
        .collect::<Vec<_>>();
    let recipe_count = recipes.len();
    let filtered_selection = recipes
        .iter()
        .position(|(index, _)| *index == app.recipe_selection);
    let selected = app.workspace.recipes.get(app.recipe_selection);
    let chunks = if area.width >= 110 {
        Layout::horizontal([Constraint::Percentage(42), Constraint::Percentage(58)]).split(area)
    } else {
        Layout::vertical([Constraint::Min(4), Constraint::Length(12)]).split(area)
    };
    let list = Layout::vertical([Constraint::Length(1), Constraint::Min(3)]).split(chunks[0]);
    let visible_rows = usize::from(list[1].height.saturating_sub(3));
    let viewport =
        yoctui_model::centered_viewport_range(filtered_selection, recipe_count, visible_rows);
    frame.render_widget(
        Paragraph::new(search_line(
            app,
            &app.metadata_query,
            app.metadata_searching,
            filtered_selection,
            recipe_count,
            SearchNavigation::Results,
            SearchExit::Done,
            list[0].width,
        )),
        list[0],
    );
    frame.render_widget(
        Table::new(
            recipes
                .into_iter()
                .skip(viewport.start)
                .take(viewport.len())
                .map(|(index, recipe)| {
                    Row::new(vec![
                        Cell::from(recipe.name.as_str()),
                        Cell::from(recipe.version.as_deref().unwrap_or("?")),
                        Cell::from(recipe.preferred_version.as_deref().unwrap_or("?")),
                        Cell::from(recipe.layer.as_deref().unwrap_or("?")),
                        Cell::from(
                            recipe
                                .append_count
                                .map_or_else(|| "?".into(), |count| count.to_string()),
                        ),
                        Cell::from(recipe_workspace_state(app, recipe)),
                        Cell::from(recipe_build_state(app, &recipe.name)),
                    ])
                    .style(selected_style(app, index == app.recipe_selection))
                }),
            [
                Constraint::Min(12),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(10),
                Constraint::Length(4),
                Constraint::Length(11),
                Constraint::Length(10),
            ],
        )
        .header(
            Row::new([
                "Recipe",
                "Resolved",
                "Preferred",
                "Layer",
                "App",
                "Workspace",
                "Build",
            ])
            .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(format!(
                    "Recipes (shown: {} of {})",
                    recipe_count,
                    app.workspace.recipes.len()
                ))
                .borders(Borders::ALL),
        ),
        list[1],
    );
    let detail = selected.map_or_else(
        || "No recipes supplied by the backend.".into(),
        |recipe| recipe_inspector(app, recipe),
    );
    frame.render_widget(
        Paragraph::new(detail)
            .block(
                Block::default()
                    .title("Recipe preview · [/] scroll")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false })
            .scroll((app.recipe_preview_scroll.min(u16::MAX as usize) as u16, 0)),
        chunks[1],
    );
}
pub(crate) fn git_state_text(state: GitFileState) -> &'static str {
    match state {
        GitFileState::Clean => "clean",
        GitFileState::Modified => "modified",
        GitFileState::Untracked => "untracked",
        GitFileState::Ignored => "ignored/generated",
        GitFileState::Unavailable => "Git unavailable",
    }
}

pub(crate) fn layer_relationship<'a>(
    app: &'a App,
    layer: &str,
) -> Option<&'a yoctui_model::LayerRelationship> {
    app.layer_relationships
        .as_ref()?
        .layers
        .iter()
        .find(|relationship| relationship.name == layer)
}

pub(crate) fn active_build_layer(app: &App, layer: &str) -> bool {
    app.build
        .target
        .as_ref()
        .and_then(|target| {
            app.workspace
                .recipes
                .iter()
                .find(|recipe| &recipe.name == target)
        })
        .and_then(|recipe| recipe.layer.as_deref())
        .is_some_and(|recipe_layer| recipe_layer == layer)
        || app.tasks.values().any(|task| {
            app.workspace
                .recipes
                .iter()
                .find(|recipe| recipe.name == task.recipe)
                .and_then(|recipe| recipe.layer.as_deref())
                .is_some_and(|recipe_layer| recipe_layer == layer)
        })
}

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

pub(crate) fn layer_browser(frame: &mut Frame, app: &App, browser: &LayerBrowser, area: Rect) {
    let left_width = layer_browser_left_width(browser, area.width);
    let chunks =
        Layout::horizontal([Constraint::Length(left_width), Constraint::Min(1)]).split(area);
    let layer_height = app.workspace.layers.len().saturating_add(2).min(8) as u16;
    let left =
        Layout::vertical([Constraint::Length(layer_height), Constraint::Min(3)]).split(chunks[0]);
    let palette = ThemePalette::for_app(app);
    let configured = app
        .workspace
        .layers
        .iter()
        .filter(|layer| {
            matches_metadata(
                &app.metadata_query,
                &[layer.name.as_str(), layer.path.to_str().unwrap_or("")],
            )
        })
        .collect::<Vec<_>>();
    let configured_selection = configured
        .iter()
        .position(|layer| layer.name == browser.layer);
    let configured_viewport = yoctui_model::centered_viewport_range(
        configured_selection,
        configured.len(),
        usize::from(left[0].height.saturating_sub(3)).max(1),
    );
    let configured_rows = configured[configured_viewport].iter().map(|layer| {
        let relationship = layer_relationship(app, &layer.name);
        let compatibility = relationship.map_or("?", |value| {
            if value.compatible.is_empty() {
                "-"
            } else {
                "yes"
            }
        });
        let active = active_build_layer(app, &layer.name);
        let style = if layer.name == browser.layer {
            palette.selected()
        } else if active {
            palette.role(palette.success, Modifier::BOLD)
        } else {
            Style::default()
        };
        Row::new([
            if active {
                format!("▪ {}", layer.name)
            } else {
                format!("  {}", layer.name)
            },
            layer
                .priority
                .map_or_else(|| "?".into(), |priority| priority.to_string()),
            compatibility.into(),
        ])
        .style(style)
    });
    frame.render_widget(
        Table::new(
            configured_rows,
            [
                Constraint::Min(8),
                Constraint::Length(4),
                Constraint::Length(6),
            ],
        )
        .header(Row::new(["Configured layers", "Pri", "Compat"]).style(Style::default().bold()))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(palette.base())
                .border_style(palette.focus()),
        ),
        left[0],
    );

    let query = app.metadata_query.to_ascii_lowercase();
    let entries = browser
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            query.is_empty()
                || entry
                    .path
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .contains(&query)
        })
        .collect::<Vec<_>>();
    let filtered_selection = entries
        .iter()
        .position(|(index, _)| *index == browser.selection);
    let tree = Layout::vertical([Constraint::Length(1), Constraint::Min(3)]).split(left[1]);
    let visible_rows = usize::from(tree[1].height.saturating_sub(2));
    let viewport = yoctui_model::variable_height_window(
        std::iter::repeat_n(1, entries.len()),
        filtered_selection,
        visible_rows,
    );
    frame.render_widget(
        Paragraph::new(search_line(
            app,
            &app.metadata_query,
            app.metadata_searching,
            filtered_selection,
            entries.len(),
            SearchNavigation::Results,
            SearchExit::Done,
            tree[0].width,
        )),
        tree[0],
    );
    let title = format!(
        "{} tree | hidden {} | rows {}-{} of {}{}{}",
        browser.layer,
        if browser.show_hidden { "on" } else { "off" },
        viewport.start.saturating_add(usize::from(viewport.end > 0)),
        viewport.end,
        browser.entries.len(),
        if browser.tree_truncated {
            " | bounded"
        } else {
            ""
        },
        if browser.cycle_entries > 0 {
            " | cycle rejected"
        } else {
            ""
        }
    );
    let unicode = app.preferences.symbols == SymbolPreference::Unicode;
    let visible_entries = entries
        .get(viewport.start..viewport.end)
        .unwrap_or_default();
    match layer_tree_widget_projection(browser, visible_entries, unicode, !query.is_empty())
        .and_then(|projection| {
            let LayerTreeWidgetProjection {
                items,
                selected,
                opened,
            } = projection;
            let widget = Tree::new(&items)?
                .block(Block::default().title(title.clone()).borders(Borders::ALL))
                .style(palette.base())
                .highlight_style(selected_style(app, true))
                .highlight_symbol("")
                .node_open_symbol(if unicode { "▾ " } else { "- " })
                .node_closed_symbol(if unicode { "▸ " } else { "+ " })
                .node_no_children_symbol("");
            let mut state = TreeState::default();
            for path in opened {
                state.open(path);
            }
            if let Some(path) = selected {
                state.select(path);
            }
            frame.render_stateful_widget(widget, tree[1], &mut state);
            Ok(())
        }) {
        Ok(()) => {}
        Err(_) => frame.render_widget(
            Paragraph::new("Layer tree unavailable: duplicate or malformed path identity.")
                .block(Block::default().title(title).borders(Borders::ALL)),
            tree[1],
        ),
    }

    let info_open = browser.inspector_mode != LayerInspectorMode::Preview;
    let right = Layout::vertical([
        Constraint::Length(if info_open { 9 } else { 3 }),
        Constraint::Min(5),
    ])
    .split(chunks[1]);
    let info = if info_open {
        layer_inspector_text(app, browser)
    } else {
        Text::from(format!(
            "{}\nPress i for file/layer information",
            browser.selected_entry().map_or_else(
                || "No file selected".into(),
                |entry| {
                    if entry.path.is_absolute() {
                        entry.path.display().to_string()
                    } else {
                        browser.root.join(&entry.path).display().to_string()
                    }
                }
            )
        ))
    };
    frame.render_widget(
        Paragraph::new(info)
            .block(
                Block::default()
                    .title(if info_open {
                        "File information · i hide"
                    } else {
                        "File information · i show"
                    })
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false }),
        right[0],
    );
    let mut preview_browser = browser.clone();
    preview_browser.inspector_mode = LayerInspectorMode::Preview;
    frame.render_widget(
        Paragraph::new(layer_inspector_text(app, &preview_browser))
            .block(
                Block::default()
                    .title(if browser.preview_focused {
                        "File preview focused · ↑/↓ scroll · ← tree · e edit"
                    } else {
                        "File preview · → focus · e edit"
                    })
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false })
            .scroll((browser.preview_scroll.min(u16::MAX as usize) as u16, 0)),
        right[1],
    );
}

pub(crate) fn layers(frame: &mut Frame, app: &App, area: Rect) {
    let layers = app
        .workspace
        .layers
        .iter()
        .filter(|layer| {
            matches_metadata(
                &app.metadata_query,
                &[layer.name.as_str(), layer.path.to_str().unwrap_or("")],
            )
        })
        .collect::<Vec<_>>();
    let layer_count = layers.len();
    let selected = layers.get(app.layer_selection).copied();
    let recipes = selected.map_or_else(Vec::new, |layer| {
        let mut recipes = app
            .workspace
            .recipes
            .iter()
            .filter(|recipe| recipe.layer.as_deref() == Some(layer.name.as_str()))
            .collect::<Vec<_>>();
        recipes.sort_by(|left, right| left.name.cmp(&right.name));
        recipes
    });
    let workspace = Layout::vertical([Constraint::Length(1), Constraint::Min(3)]).split(area);
    frame.render_widget(
        Paragraph::new(search_line(
            app,
            &app.metadata_query,
            app.metadata_searching,
            (layer_count > 0).then_some(app.layer_selection),
            layer_count,
            SearchNavigation::Results,
            SearchExit::Done,
            workspace[0].width,
        )),
        workspace[0],
    );
    let chunks = Layout::horizontal([Constraint::Percentage(48), Constraint::Percentage(52)])
        .split(workspace[1]);
    let visible_layer_rows = usize::from(chunks[0].height.saturating_sub(3));
    let layer_viewport = yoctui_model::centered_viewport_range(
        (layer_count > 0).then_some(app.layer_selection),
        layer_count,
        visible_layer_rows,
    );
    frame.render_widget(
        Table::new(
            layers
                .into_iter()
                .enumerate()
                .skip(layer_viewport.start)
                .take(layer_viewport.len())
                .map(|(index, layer)| {
                    Row::new(vec![
                        Cell::from(format!("▸ {}", layer.name)),
                        Cell::from(layer.path.display().to_string()),
                        Cell::from(
                            layer
                                .priority
                                .map_or_else(String::new, |priority| priority.to_string()),
                        ),
                    ])
                    .style({
                        let mut style = selected_style(app, index == app.layer_selection);
                        let palette = ThemePalette::for_app(app);
                        if index != app.layer_selection {
                            style = style.fg(palette.success);
                        }
                        if palette.attribute_only {
                            style = style.add_modifier(Modifier::BOLD);
                        }
                        style
                    })
                }),
            [
                Constraint::Percentage(32),
                Constraint::Percentage(53),
                Constraint::Percentage(15),
            ],
        )
        .header(
            Row::new(["Layer", "Path", "Priority"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(format!(
                    "Active layer tree (shown: {} of {})",
                    layer_count,
                    app.workspace.layers.len()
                ))
                .borders(Borders::ALL),
        ),
        chunks[0],
    );
    frame.render_widget(
        Table::new(
            recipes.iter().map(|recipe| {
                Row::new(vec![
                    Cell::from(recipe.name.as_str()),
                    Cell::from(recipe.version.as_deref().unwrap_or("")),
                ])
            }),
            [Constraint::Percentage(68), Constraint::Percentage(32)],
        )
        .header(
            Row::new(["Recipe in selected layer", "Version"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(selected.map_or_else(
                    || "Layer recipes".into(),
                    |layer| format!("Recipes: {} ({})", layer.name, recipes.len()),
                ))
                .borders(Borders::ALL),
        ),
        chunks[1],
    );
}
