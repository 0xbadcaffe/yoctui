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
