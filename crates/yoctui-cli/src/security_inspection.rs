//! Security inspection.
use super::*;

pub(crate) fn security_capability_input(
    app: &App,
    build_directory: PathBuf,
    path_directories: Vec<PathBuf>,
) -> std::result::Result<SecurityCapabilityInput, String> {
    let selected_recipe = app
        .workspace
        .recipes
        .get(app.recipe_selection)
        .and_then(|recipe| {
            recipe.file.clone().map(|file| {
                SecurityScope::Recipe(RecipeIdentity {
                    name: recipe.name.clone(),
                    file,
                })
            })
        });
    let selected_image = app.build.target.clone().and_then(|target| {
        let machine = app.workspace.variables.get("MACHINE")?.clone();
        let distro = app.workspace.variables.get("DISTRO")?.clone();
        Some(SecurityScope::Image {
            target,
            machine,
            distro,
        })
    });
    let mut available_scopes = [selected_recipe, selected_image]
        .into_iter()
        .flatten()
        .filter(SecurityScope::is_valid)
        .collect::<Vec<_>>();
    available_scopes.dedup();
    let scope = app
        .security
        .scope
        .clone()
        .filter(|scope| available_scopes.contains(scope))
        .or_else(|| available_scopes.first().cloned())
        .ok_or_else(|| {
            "Security needs an exact selected recipe provider or image target with MACHINE and DISTRO"
                .to_owned()
        })?;
    let reported_tasks = app
        .recipe_metadata
        .get(scope.target())
        .and_then(|metadata| metadata.tasks.clone())
        .unwrap_or_default();
    let explicit_paths = |names: &[&str]| {
        names
            .iter()
            .filter_map(|name| app.workspace.variables.get(*name))
            .map(PathBuf::from)
            .collect::<Vec<_>>()
    };
    let cve_roots = explicit_paths(&["CVE_CHECK_REPORT_ROOT", "CVE_CHECK_DIR", "DEPLOY_DIR_IMAGE"]);
    let sbom_roots = explicit_paths(&["DEPLOY_DIR_SPDX", "DEPLOY_DIR_SBOM", "DEPLOY_DIR_IMAGE"]);
    let image_build_emits_sbom = ["INHERIT", "IMAGE_CLASSES"]
        .iter()
        .filter_map(|name| app.workspace.variables.get(*name))
        .flat_map(|value| value.split_whitespace())
        .any(|value| {
            matches!(
                value,
                "create-spdx" | "create-spdx-2.0" | "create_sbom" | "create-sbom"
            )
        });
    Ok(SecurityCapabilityInput {
        release: app.workspace.release.clone(),
        build_directory,
        scope,
        available_scopes,
        reported_tasks,
        image_build_emits_sbom,
        cve_roots,
        sbom_roots,
        path_directories,
    })
}

pub(crate) async fn open_security_url(app: &mut App, opener: Option<PathBuf>, url: String) {
    if !url.starts_with("https://") || url.chars().any(char::is_control) {
        app.notification = Some("The selected Security advisory URL is invalid.".into());
        return;
    }
    let Some(opener) = opener else {
        app.notification =
            Some("Cannot open the Security advisory because xdg-open is unavailable.".into());
        return;
    };
    let result = tokio::task::spawn_blocking(move || {
        ProcessCommand::new(opener)
            .arg(&url)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
    })
    .await;
    match result {
        Ok(Ok(status)) if status.success() => {}
        Ok(Ok(status)) => {
            app.notification = Some(format!("xdg-open exited with {status}."));
        }
        Ok(Err(error)) => {
            app.notification = Some(format!("Could not start xdg-open: {error}"));
        }
        Err(error) => {
            app.notification = Some(format!("Security URL opener task was lost: {error}"));
        }
    }
}

pub(crate) async fn route_independent_security_effect(
    guard: &TerminalGuard,
    app: &mut App,
    coordinator: &mut SecurityCliCoordinator,
    effect: Effect,
    editor: Option<&str>,
) -> bool {
    match &effect {
        Effect::Security(SecurityEffect::StartBuild { .. }) => false,
        Effect::Security(SecurityEffect::CancelSession(id)) if !coordinator.owns_mapper(*id) => {
            false
        }
        Effect::Security(SecurityEffect::OpenPath(path)) => {
            match coordinator.revalidate_open_path(app, path).await {
                Ok(()) => open_in_editor(guard, app, path.clone(), editor).await,
                Err(message) => app.notification = Some(message),
            }
            true
        }
        Effect::Security(SecurityEffect::OpenUrl(url)) => {
            open_security_url(app, coordinator.url_opener(), url.clone()).await;
            true
        }
        Effect::Security(_) => coordinator.handle_effect(app, effect).await,
        _ => false,
    }
}
