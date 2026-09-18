//! Workspace commands.
use super::*;

pub(crate) async fn load_workspace(
    backend: Backend,
    build_dir: PathBuf,
) -> Result<yoctui_model::Workspace> {
    let mut backend = select_backend(backend, build_dir).await?;
    let result = backend.inspect_workspace().await;
    let shutdown = backend.shutdown().await;
    let workspace = result?;
    shutdown?;
    Ok(workspace)
}

pub(crate) async fn inspect_workspace(backend: Backend, build_dir: PathBuf) -> Result<()> {
    let workspace = load_workspace(backend, build_dir).await?;
    println!(
        "build directory: {}",
        workspace
            .build_dir
            .as_deref()
            .map_or_else(|| "unknown".into(), |path| path.display().to_string())
    );
    println!(
        "BitBake version: {}",
        workspace.bitbake_version.as_deref().unwrap_or("unknown")
    );
    println!(
        "Yocto/OpenEmbedded release: {}",
        workspace.release.as_deref().unwrap_or("unknown")
    );
    let coexistence = bitbake_coexistence_diagnostic(
        &workspace,
        &HostTelemetry {
            logical_cpu_count: std::thread::available_parallelism()
                .ok()
                .and_then(|count| u16::try_from(count.get()).ok()),
            load_average_milli: read_load_average(),
            ..HostTelemetry::default()
        },
    );
    for line in bitbake_coexistence_lines(&coexistence) {
        println!("{line}");
    }
    for (name, value) in workspace.variables {
        println!("{name}={value}");
    }
    Ok(())
}

pub(crate) fn bitbake_coexistence_lines(diagnostic: &BitBakeCoexistenceDiagnostic) -> Vec<String> {
    let pressure = match diagnostic.pressure {
        BitBakeCoexistencePressure::Unknown => "unknown",
        BitBakeCoexistencePressure::Nominal => "nominal",
        BitBakeCoexistencePressure::Busy => "busy",
        BitBakeCoexistencePressure::Oversubscribed => "oversubscribed",
    };
    let value =
        |value: Option<u16>| value.map_or_else(|| "unknown".into(), |value| value.to_string());
    let load = diagnostic.load_one_milli.map_or_else(
        || "unknown".into(),
        |value| format!("{:.2}", f64::from(value) / 1_000.0),
    );
    let mut lines = vec![
        format!("coexistence pressure: {pressure}"),
        format!(
            "coexistence host: logical CPUs={}, load1={load}",
            value(diagnostic.logical_cpu_count)
        ),
        format!(
            "coexistence config: BB_NUMBER_THREADS={}, PARALLEL_MAKE jobs={}",
            value(diagnostic.bitbake_threads),
            value(diagnostic.parallel_make_jobs)
        ),
    ];
    lines.extend(
        diagnostic
            .reasons
            .iter()
            .map(|reason| format!("coexistence observation: {reason}")),
    );
    if diagnostic.pressure == BitBakeCoexistencePressure::Oversubscribed
        && let Some(jobs) = diagnostic.review_example_jobs
    {
        lines.push(format!(
            "coexistence review example: BB_NUMBER_THREADS=\"{jobs}\" and PARALLEL_MAKE=\"-j {jobs}\"; verify for this workload"
        ));
    }
    lines.push("coexistence policy: read-only; Yoctui changed no BitBake configuration".into());
    lines
}

pub(crate) async fn inspect_project_profile(
    backend_kind: Backend,
    build_dir: PathBuf,
) -> Result<()> {
    let root = project_profile_root(&build_dir)
        .context("could not determine the project root for the selected build directory")?;
    let profile = load_project_profile(&root)?;
    let mut backend = select_backend(backend_kind, build_dir).await?;
    let result = async {
        let workspace = backend.inspect_workspace().await?;
        let recipes = backend.list_recipes(None).await?;
        let layers = backend.list_layers().await?;
        let mut app = App::new(1_000, 16 * 1024 * 1024);
        let _ = update(&mut app, Action::WorkspaceLoaded(workspace));
        let _ = update(&mut app, Action::RecipesLoaded(recipes));
        let _ = update(&mut app, Action::LayersLoaded(layers));
        match profile {
            Some(profile) => {
                let _ = update(&mut app, Action::ProjectProfileLoaded(profile));
                print_project_profile_summary(&app);
            }
            None => println!("project profile: absent (optional)"),
        }
        println!(
            "BitBake version: {}",
            app.workspace
                .bitbake_version
                .as_deref()
                .unwrap_or("unknown")
        );
        Ok::<(), anyhow::Error>(())
    }
    .await;
    let shutdown = backend.shutdown().await;
    result?;
    Ok(shutdown?)
}

pub(crate) fn print_project_profile_summary(app: &App) {
    for line in project_profile_summary(app) {
        println!("{line}");
    }
}

pub(crate) fn project_profile_summary(app: &App) -> Vec<String> {
    let items = yoctui_model::project_profile_items(
        &app.project_profile,
        &app.workspace,
        &app.available_images,
    );
    let mut lines = vec!["project profile: loaded".to_owned()];
    lines.extend(items.into_iter().map(|item| {
        let status = match item.status {
            yoctui_model::ProjectProfileItemStatus::Resolved => "resolved".to_owned(),
            yoctui_model::ProjectProfileItemStatus::Stale(reason) => {
                format!("stale ({reason})")
            }
            yoctui_model::ProjectProfileItemStatus::Ambiguous(count) => {
                format!("ambiguous ({count} matches)")
            }
            yoctui_model::ProjectProfileItemStatus::Unavailable(reason) => {
                format!("unavailable ({reason})")
            }
        };
        format!("profile item: {status} {:?}", item.kind)
    }));
    lines
}

pub(crate) async fn print_recipes(backend: Backend, build_dir: PathBuf) -> Result<()> {
    let mut backend = select_backend(backend, build_dir).await?;
    let result = backend.list_recipes(None).await;
    let shutdown = backend.shutdown().await;
    let recipes = result?;
    shutdown?;
    for recipe in recipes {
        println!("{} {}", recipe.name, recipe.version.unwrap_or_default());
    }
    Ok(())
}

pub(crate) async fn print_layers(backend: Backend, build_dir: PathBuf) -> Result<()> {
    let mut backend = select_backend(backend, build_dir).await?;
    let result = backend.list_layers().await;
    let shutdown = backend.shutdown().await;
    let layers = result?;
    shutdown?;
    for layer in layers {
        println!("{} {}", layer.name, layer.path.display());
    }
    Ok(())
}

pub(crate) async fn print_variable(backend: Backend, build_dir: PathBuf, name: &str) -> Result<()> {
    let _ = backend;
    let compatibility = current_daemon_compatibility(&build_dir)?;
    let planner = yoctui_bitbake::BitBakeCommandPlanner::new(
        &compatibility,
        compatibility.snapshot.generation,
        &build_dir,
    )?;
    let command = planner.get_variable(name, None)?;
    let implementation = command.implementation.clone();
    let output = run_bounded_config_query(command, &build_dir).await?;
    let value = config_value_from_authorized_output(name, &implementation, &output)?;
    println!("{name}={value}");
    Ok(())
}
