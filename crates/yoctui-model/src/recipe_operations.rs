//! Recipe operations.
use super::*;

pub(crate) fn select_first_matching_layer_entry(app: &mut App) {
    let query = app.metadata_query.to_ascii_lowercase();
    if let Some(browser) = app.layer_browser.as_mut()
        && let Some(index) = browser.entries.iter().position(|entry| {
            query.is_empty()
                || entry
                    .path
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .contains(&query)
        })
    {
        browser.selection = index;
    }
}

pub(crate) fn recipe_matches_query(recipe: &Recipe, query: &str) -> bool {
    let query = query.to_ascii_lowercase();
    query.is_empty()
        || [
            Some(recipe.name.as_str()),
            recipe.version.as_deref(),
            recipe.preferred_version.as_deref(),
            recipe.layer.as_deref(),
            recipe.file.as_ref().and_then(|path| path.to_str()),
        ]
        .into_iter()
        .flatten()
        .any(|value| value.to_ascii_lowercase().contains(&query))
}

pub(crate) fn select_first_matching_recipe(app: &mut App) {
    if let Some(index) = app
        .workspace
        .recipes
        .iter()
        .position(|recipe| recipe_matches_query(recipe, &app.metadata_query))
    {
        app.recipe_selection = index;
    }
}

pub(crate) fn begin_recipe_task_for(
    app: &mut App,
    recipe_name: &str,
    task: Option<String>,
    force: bool,
) {
    let Some(recipe) = app
        .workspace
        .recipes
        .iter()
        .find(|recipe| recipe.name == recipe_name)
    else {
        app.notification = Some("No recipe is selected for this task.".into());
        return;
    };
    if let Some(task) = task.as_deref() {
        let Some(metadata) = app.recipe_metadata.get(&recipe.name) else {
            app.notification =
                Some("Load selected recipe metadata with Enter before choosing a task.".into());
            return;
        };
        let Some(tasks) = metadata.tasks.as_ref() else {
            app.notification =
                Some("The backend cannot report tasks for the selected recipe.".into());
            return;
        };
        let canonical = format!("do_{task}");
        if !tasks
            .iter()
            .any(|candidate| candidate == task || candidate == &canonical)
        {
            app.notification = Some(format!(
                "Task {task} is not reported for recipe {}.",
                recipe.name
            ));
            return;
        }
    }
    let request = BuildRequest {
        targets: vec![recipe.name.clone()],
        task,
        force,
    };
    if let Err(error) = request.validate() {
        app.notification = Some(error.to_string());
    } else {
        open_dialog(app, Dialog::RecipeTaskConfirmation(request));
    }
}

pub(crate) fn begin_recipe_task(app: &mut App, task: Option<String>, force: bool) {
    let Some(recipe_name) = app
        .workspace
        .recipes
        .get(app.recipe_selection)
        .map(|recipe| recipe.name.clone())
    else {
        app.notification = Some("No recipe is selected for this task.".into());
        return;
    };
    begin_recipe_task_for(app, &recipe_name, task, force);
}

pub(crate) fn selected_recipe_identity(app: &App) -> Result<RecipeIdentity, &'static str> {
    let recipe = app
        .workspace
        .recipes
        .get(app.recipe_selection)
        .ok_or("No recipe is selected for Devtool status.")?;
    let file = recipe
        .file
        .clone()
        .ok_or("The selected recipe has no authoritative provider path.")?;
    if !file.is_absolute() {
        return Err("The selected recipe provider path is not absolute.");
    }
    Ok(RecipeIdentity {
        name: recipe.name.clone(),
        file,
    })
}

pub(crate) fn terminal_creation_request(
    app: &mut App,
    task: Option<&str>,
) -> Option<TerminalLaunchRequest> {
    if app.daemon.status != ClientReplicaStatus::Current {
        app.notification =
            Some("Reconnect to a current daemon replica before creating a terminal.".into());
        return None;
    }
    if !app.build_environment.connected() {
        app.notification =
            Some("Verify the build environment before creating a daemon-owned terminal.".into());
        return None;
    }
    let Some(cwd) = app.workspace.build_dir.clone() else {
        app.notification = Some("No authoritative build directory is available.".into());
        return None;
    };
    if task.is_none() {
        return Some(TerminalLaunchRequest {
            name: "build shell".into(),
            kind: TerminalCreationKind::BuildShell,
            cwd,
            program: PathBuf::from("/bin/sh"),
            arguments: Vec::new(),
        });
    }

    let task = task.expect("the build-shell case returned above");
    let Some(recipe) = app.workspace.recipes.get(app.recipe_selection) else {
        app.notification = Some(format!("Select a recipe before opening {task}."));
        return None;
    };
    let Some(tasks) = app
        .recipe_metadata
        .get(&recipe.name)
        .and_then(|metadata| metadata.tasks.as_ref())
    else {
        app.notification = Some(
            "Load authoritative recipe tasks with Enter before opening an interactive task.".into(),
        );
        return None;
    };
    let canonical = format!("do_{task}");
    if !tasks
        .iter()
        .any(|candidate| candidate == task || candidate == &canonical)
    {
        app.notification = Some(format!(
            "Task {task} is not reported for recipe {}.",
            recipe.name
        ));
        return None;
    }
    let request = BuildRequest {
        targets: vec![recipe.name.clone()],
        task: Some(task.into()),
        force: false,
    };
    if let Err(error) = request.validate() {
        app.notification = Some(error.to_string());
        return None;
    }
    Some(TerminalLaunchRequest {
        name: format!("{task}:{}", recipe.name),
        kind: if task == "devshell" {
            TerminalCreationKind::Devshell
        } else {
            TerminalCreationKind::Menuconfig
        },
        cwd,
        program: PathBuf::from("/usr/bin/env"),
        arguments: vec![
            "bitbake".into(),
            recipe.name.clone(),
            "-c".into(),
            task.into(),
        ],
    })
}

pub(crate) fn open_terminal_launch(app: &mut App, request: TerminalLaunchRequest) {
    open_dialog(
        app,
        Dialog::TerminalLaunch(TerminalLaunchDialog {
            request,
            destination: TerminalLaunchDestination::Embedded,
            output_must_not_exist: None,
        }),
    );
}

pub(crate) fn begin_terminal_creation(app: &mut App, task: Option<&str>) {
    if let Some(request) = terminal_creation_request(app, task) {
        open_terminal_launch(app, request);
    }
}

pub(crate) fn devtool_terminal_request(
    app: &mut App,
    edit_recipe: bool,
) -> Option<TerminalLaunchRequest> {
    let identity = match selected_recipe_identity(app) {
        Ok(identity) => identity,
        Err(message) => {
            app.notification = Some(message.to_owned());
            return None;
        }
    };
    let Some(status) = app.devtool_statuses.get(&identity) else {
        app.notification =
            Some("Refresh Devtool status with t before starting an interactive session.".into());
        return None;
    };
    if status.capability != DevtoolCapability::Available || status.error.is_some() {
        app.notification = Some(
            status
                .disabled_reason(DevtoolAction::ModifyOrEdit)
                .unwrap_or_else(|| "Devtool interactive editing is unavailable.".into()),
        );
        return None;
    }
    if edit_recipe {
        let Some(cwd) = app.workspace.build_dir.clone() else {
            app.notification = Some("No authoritative build directory is available.".into());
            return None;
        };
        return Some(TerminalLaunchRequest {
            name: format!("devtool edit-recipe {}", identity.name),
            kind: TerminalCreationKind::Utility,
            cwd,
            program: PathBuf::from("/usr/bin/env"),
            arguments: vec!["devtool".into(), "edit-recipe".into(), identity.name],
        });
    }
    let source_path = match &status.workspace {
        DevtoolWorkspace::Present { source_path, .. } => source_path.clone(),
        DevtoolWorkspace::NotMember | DevtoolWorkspace::MissingDirectory { .. } => {
            app.notification = Some(
                "Run Devtool modify first; no authoritative workspace source is available.".into(),
            );
            return None;
        }
    };
    Some(TerminalLaunchRequest {
        name: format!("devtool workspace {}", identity.name),
        kind: TerminalCreationKind::DevtoolShell,
        cwd: source_path,
        program: PathBuf::from("/bin/sh"),
        arguments: Vec::new(),
    })
}
