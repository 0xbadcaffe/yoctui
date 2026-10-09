use super::*;

const DEVTOOL_STATUS_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) struct DevtoolStatusOperation {
    identity: RecipeIdentity,
    modify_completion: bool,
    handle: tokio::task::JoinHandle<std::result::Result<yoctui_model::DevtoolStatus, String>>,
}

async fn status_with_deadline(
    future: impl std::future::Future<Output = yoctui_model::DevtoolStatus>,
    timeout: Duration,
) -> std::result::Result<yoctui_model::DevtoolStatus, String> {
    tokio::time::timeout(timeout, future).await.map_err(|_| {
        "Devtool status timed out; refresh when BitBake is free. Existing workspace status was retained.".into()
    })
}

fn spawn_status(
    build_dir: PathBuf,
    identity: RecipeIdentity,
    authority: Option<yoctui_model::DaemonCompatibilitySnapshot>,
) -> tokio::task::JoinHandle<std::result::Result<yoctui_model::DevtoolStatus, String>> {
    tokio::spawn(async move {
        status_with_deadline(
            inspect_devtool_status_with_authority(&build_dir, identity, authority),
            DEVTOOL_STATUS_TIMEOUT,
        )
        .await
    })
}

impl InteractiveRuntime {
    pub(super) fn begin_selected_devtool_status(&mut self) {
        if self.devtool_status_operation.is_some() {
            self.app.notification = Some("Devtool status inspection is already running.".into());
            return;
        }
        let Some(Effect::InspectDevtoolStatus(identity)) =
            compatibility_workspace_action(&mut self.app, Action::BeginSelectedRecipeDevtoolStatus)
        else {
            return;
        };
        let effect = Effect::InspectDevtoolStatus(identity.clone());
        if submit_daemon_effect(&mut self.daemon_runtime, &mut self.app, &effect) == Some(true) {
            return;
        }
        let build_dir = self.session_build_dir.clone();
        let authority = self.app.workspace_compatibility.authority().cloned();
        let handle = spawn_status(build_dir, identity.clone(), authority);
        self.devtool_status_operation = Some(DevtoolStatusOperation {
            identity,
            handle,
            modify_completion: false,
        });
    }

    pub(super) fn begin_devtool_modify_completion(&mut self, identity: RecipeIdentity) {
        if let Some(operation) = self.devtool_status_operation.take() {
            operation.handle.abort();
        }
        let build_dir = self.session_build_dir.clone();
        let authority = self.app.workspace_compatibility.authority().cloned();
        let handle = spawn_status(build_dir, identity.clone(), authority);
        self.devtool_status_operation = Some(DevtoolStatusOperation {
            identity,
            handle,
            modify_completion: true,
        });
        self.app.notification =
            Some("Devtool modify completed; checking workspace in the background.".into());
    }

    pub(super) async fn poll_devtool_status(&mut self) -> bool {
        if !self
            .devtool_status_operation
            .as_ref()
            .is_some_and(|operation| operation.handle.is_finished())
        {
            return false;
        }
        let operation = self
            .devtool_status_operation
            .take()
            .expect("finished Devtool status operation");
        let status = match operation.handle.await {
            Ok(Ok(status)) => status,
            result => {
                let message = match result {
                    Ok(Err(message)) => message,
                    Err(error) => format!("Devtool status task failed: {error}"),
                    _ => unreachable!(),
                };
                self.app.notification = Some(format!("{}: {message}", operation.identity.name));
                return true;
            }
        };
        if operation.modify_completion
            && matches!(self.app.screen, Screen::Recipes | Screen::Devtool)
            && self.app.active_dialog().is_none()
        {
            apply_completed_devtool_modify_status(&mut self.app, status).await;
        } else {
            let _ =
                compatibility_workspace_action(&mut self.app, Action::DevtoolStatusLoaded(status));
        }
        true
    }

    pub(super) async fn stop_devtool_status(&mut self) {
        if let Some(operation) = self.devtool_status_operation.take() {
            operation.handle.abort();
            let _ = operation.handle.await;
        }
    }
}

#[cfg(test)]
#[path = "../tests/interactive_runtime/devtool_status_operation.rs"]
mod tests;
