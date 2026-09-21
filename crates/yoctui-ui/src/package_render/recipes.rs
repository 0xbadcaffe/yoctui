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
