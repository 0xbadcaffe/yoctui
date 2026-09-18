//! Kernel inspection.
use super::*;

pub(crate) async fn inspect_kernel_workbench(app: &mut App, backend: &mut dyn BitBakeBackend) {
    let target = "virtual/kernel".to_owned();
    let metadata = match backend.get_recipe_metadata(target.clone()).await {
        Ok(metadata) => metadata,
        Err(error) => {
            let _ = compatibility_workspace_action(app, Action::KernelFailed(error.to_string()));
            return;
        }
    };
    let mut roots = Vec::new();
    let mut limitations = Vec::new();
    let mut provider = None;
    for variable in ["FILE", "S", "B", "WORKDIR"] {
        match backend
            .get_variable(variable.into(), Some(target.clone()))
            .await
        {
            Ok(value) => {
                let Some(value) = value.value.filter(|value| !value.trim().is_empty()) else {
                    limitations.push(format!("{variable} was not reported for {target}."));
                    continue;
                };
                let path = PathBuf::from(value);
                if variable == "FILE" {
                    if path.is_absolute() {
                        provider = Some(path);
                    } else {
                        limitations.push("Kernel provider FILE was not absolute.".into());
                    }
                } else if path.is_absolute() {
                    roots.push(path);
                } else {
                    limitations.push(format!("Kernel {variable} was not absolute."));
                }
            }
            Err(error) => limitations.push(format!("Could not query kernel {variable}: {error}")),
        }
    }
    if let Some(deploy) = app.workspace.variables.get("DEPLOY_DIR_IMAGE") {
        roots.push(PathBuf::from(deploy));
    }
    let scan = tokio::task::spawn_blocking(move || PlatformArtifactAdapter.scan(roots)).await;
    match scan {
        Ok(Ok(scan)) => {
            limitations.extend(scan.limitations);
            let _ = compatibility_workspace_action(
                app,
                Action::KernelLoaded(PlatformInventory {
                    component: PlatformComponent::Kernel,
                    target,
                    provider,
                    tasks: metadata.tasks.unwrap_or_default(),
                    roots: scan.roots,
                    files: scan.files,
                    dtc: scan.dtc,
                    limitations,
                }),
            );
        }
        Ok(Err(error)) => {
            let _ = compatibility_workspace_action(app, Action::KernelFailed(error.to_string()));
        }
        Err(error) => {
            let _ = compatibility_workspace_action(
                app,
                Action::KernelFailed(format!("artifact scanner did not complete: {error}")),
            );
        }
    }
}
