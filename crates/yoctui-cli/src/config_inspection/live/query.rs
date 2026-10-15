use super::*;
use tokio::sync::oneshot;

pub(super) struct FreshConfiguration {
    pub workspace: yoctui_model::Workspace,
    pub detail: Option<(VariableIdentity, std::result::Result<VariableValue, String>)>,
}

pub(super) async fn read(
    build_dir: PathBuf,
    source_dir: Option<PathBuf>,
    identity: Option<VariableIdentity>,
    mut cancel: oneshot::Receiver<()>,
) -> Result<Option<FreshConfiguration>> {
    let mut environment = tokio::select! {
        _ = &mut cancel => return Ok(None),
        environment = crate::interactive_runtime::metadata_environment(&build_dir, source_dir.as_deref()) => environment?,
    };
    environment.insert("YOCTUI_BRIDGE_FAST_RECIPE_INSPECTION".into(), "1".into());
    let python = env::var("PYTHON").unwrap_or_else(|_| "python3".into());
    let mut backend = tokio::select! {
        _ = &mut cancel => return Ok(None),
        backend = spawn_configured_bridge(&python, build_dir.clone(), Some(environment)) => backend?,
    }.with_configuration_inspection_scope();
    let query = async {
        let workspace = backend.inspect_workspace().await?;
        let expected = build_dir
            .canonicalize()
            .context("selected build directory is unavailable")?;
        anyhow::ensure!(
            workspace
                .build_dir
                .as_ref()
                .and_then(|p| p.canonicalize().ok())
                == Some(expected),
            "configuration query returned a different build directory"
        );
        let detail = if let Some(identity) = identity {
            let value = backend
                .get_variable(identity.name.clone(), identity.recipe.clone())
                .await
                .map_err(|error| error.to_string());
            Some((identity, value))
        } else {
            None
        };
        Ok(FreshConfiguration { workspace, detail })
    };
    let result: Result<Option<FreshConfiguration>> = tokio::select! {
        _ = &mut cancel => Ok(None),
        result = tokio::time::timeout(Duration::from_secs(120), query) => {
            match result {
                Ok(result) => result.map(Some),
                Err(error) => Err(anyhow::Error::new(error).context("configuration inspection timed out after 120 seconds")),
            }
        }
    };
    if matches!(&result, Ok(Some(_)))
        && tokio::time::timeout(Duration::from_secs(5), backend.shutdown())
            .await
            .is_ok_and(|result| result.is_ok())
    {
        return result;
    }
    #[cfg(unix)]
    backend.interrupt_metadata().await;
    result
}
