//! Daemon startup.
use super::*;

pub(crate) struct StartupWorkspace {
    pub workspace: yoctui_model::Workspace,
    pub task_metadata: Vec<(yoctui_model::RecipeIdentity, Vec<String>)>,
}

#[cfg(unix)]
pub(crate) async fn inspect_daemon_startup_workspace(
    startup_environment: &BTreeMap<String, String>,
    compatibility: Option<yoctui_model::DaemonCompatibilitySnapshot>,
    preferred_targets: Vec<String>,
    mut cancelled: tokio::sync::oneshot::Receiver<()>,
) -> Result<Option<StartupWorkspace>> {
    let Some(compatibility) = compatibility else {
        return Ok(None);
    };
    let Some(build_dir) = compatibility
        .snapshot
        .environment
        .build_directory
        .value()
        .cloned()
    else {
        return Ok(None);
    };
    let python = startup_environment
        .get("PYTHON")
        .map(String::as_str)
        .unwrap_or("python3");
    let startup = spawn_configured_bridge_with_compatibility_at_priority(
        python,
        build_dir,
        Some(startup_environment.clone()),
        compatibility,
        BridgeProcessPriority::Background,
    );
    let mut backend = tokio::select! {
        biased;
        _ = &mut cancelled => return Err(anyhow::anyhow!("startup metadata scan cancelled")),
        result = tokio::time::timeout(Duration::from_secs(30), startup) =>
            result.context("startup metadata bridge handshake timed out")?
                .context("could not start the daemon-owned startup metadata bridge")?,
    };
    let result = tokio::select! {
        biased;
        _ = &mut cancelled => Err(anyhow::anyhow!("startup metadata scan cancelled")),
        result = tokio::time::timeout(Duration::from_secs(600), async {
            let mut workspace = backend.inspect_workspace().await?;
            workspace.recipes = backend.list_recipes(None).await?;
            workspace.layers = backend.list_layers().await?;
            let mut candidates = preferred_targets;
            // The default Poky validation image is a fallback only when it
            // actually exists in this parsed inventory. No task is inferred
            // from its name: the recipe datastore below supplies the proof.
            if candidates.is_empty() {
                candidates.push("core-image-minimal".into());
            }
            candidates.push("virtual/kernel".into());
            candidates.truncate(3);
            let mut task_metadata = Vec::new();
            for candidate in candidates {
                if candidate != "virtual/kernel"
                    && !workspace.recipes.iter().any(|recipe| recipe.name == candidate) {
                    continue;
                }
                match backend.get_recipe_metadata(candidate.clone()).await {
                    Ok(metadata) => {
                        if let (Some(tasks), Some(sources)) = (metadata.tasks, metadata.sources)
                            && tasks.len() <= 1024
                            && let Some(file) = sources.first()
                        {
                            task_metadata.push((yoctui_model::RecipeIdentity { name: candidate, file: file.clone() }, tasks));
                        }
                    }
                    Err(error) => tracing::warn!(%error, recipe = %candidate, "task metadata unavailable; capability remains unknown"),
                }
            }
            Ok::<_, anyhow::Error>(Some(StartupWorkspace { workspace, task_metadata }))
        }) => result.context("startup metadata scan exceeded ten minutes").and_then(|result| result),
    };
    if result.is_err()
        || !matches!(
            tokio::time::timeout(Duration::from_secs(5), backend.shutdown()).await,
            Ok(Ok(()))
        )
    {
        backend.interrupt_metadata().await;
    }
    result
}
