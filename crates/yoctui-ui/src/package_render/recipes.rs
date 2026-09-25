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

pub(crate) fn devtool_workspace(frame: &mut Frame, app: &App, area: Rect) {
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
                    recipe.layer.as_deref().unwrap_or(""),
                    recipe
                        .file
                        .as_ref()
                        .and_then(|path| path.to_str())
                        .unwrap_or(""),
                    devtool_compact_state(app, recipe).0,
                ],
            )
        })
        .collect::<Vec<_>>();
    let filtered_selection = recipes
        .iter()
        .position(|(index, _)| *index == app.recipe_selection);
    let chunks = if area.width >= 100 {
        Layout::horizontal([Constraint::Percentage(48), Constraint::Percentage(52)]).split(area)
    } else {
        Layout::vertical([Constraint::Min(7), Constraint::Length(16)]).split(area)
    };
    let list = Layout::vertical([Constraint::Length(1), Constraint::Min(3)]).split(chunks[0]);
    let visible_rows = usize::from(list[1].height.saturating_sub(3));
    let viewport = yoctui_model::centered_viewport_range(
        filtered_selection,
        recipes.len(),
        visible_rows,
    );
    frame.render_widget(
        Paragraph::new(search_line(
            app,
            &app.metadata_query,
            app.metadata_searching,
            filtered_selection,
            recipes.len(),
            SearchNavigation::Results,
            SearchExit::Done,
            list[0].width,
        )),
        list[0],
    );
    frame.render_widget(
        Table::new(
            recipes
                .iter()
                .skip(viewport.start)
                .take(viewport.len())
                .map(|(index, recipe)| {
                    let (workspace, git) = devtool_compact_state(app, recipe);
                    Row::new(vec![
                        Cell::from(recipe.name.as_str()),
                        Cell::from(recipe.layer.as_deref().unwrap_or("?")),
                        Cell::from(workspace),
                        Cell::from(git),
                        Cell::from(recipe_build_state(app, &recipe.name)),
                    ])
                    .style(selected_style(app, *index == app.recipe_selection))
                }),
            [
                Constraint::Min(14),
                Constraint::Length(12),
                Constraint::Length(12),
                Constraint::Length(10),
                Constraint::Length(10),
            ],
        )
        .header(
            Row::new(["Recipe", "Layer", "Workspace", "Git", "Build"])
                .style(Style::default().add_modifier(Modifier::BOLD)),
        )
        .block(
            Block::default()
                .title(format!(
                    "Devtool Workspace (shown: {} of {})",
                    recipes.len(),
                    app.workspace.recipes.len()
                ))
                .borders(Borders::ALL),
        ),
        list[1],
    );
    let detail = app
        .workspace
        .recipes
        .get(app.recipe_selection)
        .map_or_else(
            || "No authoritative recipes are available.".into(),
            |recipe| devtool_workflow_detail(app, recipe),
        );
    frame.render_widget(
        Paragraph::new(detail)
            .block(
                Block::default()
                    .title("Selected recipe · development workflow")
                    .borders(Borders::ALL),
            )
            .wrap(Wrap { trim: false })
            .scroll((app.recipe_preview_scroll.min(u16::MAX as usize) as u16, 0)),
        chunks[1],
    );
}

fn devtool_compact_state<'a>(app: &'a App, recipe: &'a Recipe) -> (&'a str, &'a str) {
    let Some(file) = recipe.file.as_ref().filter(|path| path.is_absolute()) else {
        return ("unavailable", "-");
    };
    let identity = RecipeIdentity {
        name: recipe.name.clone(),
        file: file.clone(),
    };
    if app.devtool_status_loading.contains(&identity) {
        return ("loading", "-");
    }
    let Some(status) = app.devtool_statuses.get(&identity) else {
        return ("not inspected", "-");
    };
    if status.error.is_some() || status.capability != DevtoolCapability::Available {
        return ("failed", "-");
    }
    match (&status.workspace, &status.git) {
        (DevtoolWorkspace::NotMember, _) => ("not started", "-"),
        (DevtoolWorkspace::MissingDirectory { .. }, _) => ("source missing", "-"),
        (
            DevtoolWorkspace::Present { .. },
            DevtoolGitState::Available {
                modified,
                untracked,
                conflicted,
                ..
            },
        ) if modified + untracked + conflicted == 0 => ("ready", "clean"),
        (DevtoolWorkspace::Present { .. }, DevtoolGitState::Available { .. }) => {
            ("ready", "changed")
        }
        (DevtoolWorkspace::Present { .. }, _) => ("ready", "unavailable"),
    }
}

fn devtool_workflow_detail(app: &App, recipe: &Recipe) -> String {
    let (workspace, git) = devtool_compact_state(app, recipe);
    let source = recipe
        .file
        .as_ref()
        .filter(|path| path.is_absolute())
        .and_then(|file| {
            app.devtool_statuses.get(&RecipeIdentity {
                name: recipe.name.clone(),
                file: file.clone(),
            })
        })
        .and_then(|status| match &status.workspace {
            DevtoolWorkspace::Present { source_path, .. }
            | DevtoolWorkspace::MissingDirectory { source_path } => Some(source_path),
            DevtoolWorkspace::NotMember => None,
        });
    format!(
        "Recipe: {}\nProvider: {}\nWorkspace: {workspace}\nSource: {}\nGit: {git}\nBuild: {}\n\n1  Start/refresh workspace       Enter / d\n2  Edit source                   d / e\n3  Build workspace recipe        b\n4  Deploy build with SSH/SCP     P\n5  Create/update patches         u\n6  Finish into configured layer  F\n\nWorkspace shell: s   GitUI: G   Reset: D\n\n{}",
        recipe.name,
        recipe
            .file
            .as_ref()
            .map_or_else(|| "unavailable".into(), |path| path.display().to_string()),
        source.map_or_else(|| "not available".into(), |path| path.display().to_string()),
        recipe_build_state(app, &recipe.name),
        recipe_workspace_state(app, recipe),
    )
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
