use super::*;

const DEVTOOL_STATUS_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) struct DevtoolStatusOperation {
    identity: RecipeIdentity,
    handle: tokio::task::JoinHandle<yoctui_model::DevtoolStatus>,
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
        let build_dir = self.session_build_dir.clone();
        let authority = self.app.workspace_compatibility.authority().cloned();
        let worker_identity = identity.clone();
        let handle = tokio::spawn(async move {
            match tokio::time::timeout(
                DEVTOOL_STATUS_TIMEOUT,
                inspect_devtool_status_with_authority(
                    &build_dir,
                    worker_identity.clone(),
                    authority,
                ),
            )
            .await
            {
                Ok(status) => status,
                Err(_) => yoctui_model::DevtoolStatus {
                    identity: worker_identity,
                    capability: yoctui_model::DevtoolCapability::Unavailable {
                        reason: "Devtool status timed out after 30 seconds.".into(),
                    },
                    workspace: DevtoolWorkspace::NotMember,
                    git: yoctui_model::DevtoolGitState::NotApplicable,
                    error: None,
                },
            }
        });
        self.devtool_status_operation = Some(DevtoolStatusOperation { identity, handle });
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
            Ok(status) => status,
            Err(error) => yoctui_model::DevtoolStatus {
                identity: operation.identity,
                capability: yoctui_model::DevtoolCapability::Unavailable {
                    reason: format!("Devtool status task failed: {error}"),
                },
                workspace: DevtoolWorkspace::NotMember,
                git: yoctui_model::DevtoolGitState::NotApplicable,
                error: None,
            },
        };
        let _ = compatibility_workspace_action(&mut self.app, Action::DevtoolStatusLoaded(status));
        true
    }

    pub(super) async fn stop_devtool_status(&mut self) {
        if let Some(operation) = self.devtool_status_operation.take() {
            operation.handle.abort();
            let _ = operation.handle.await;
        }
    }
}
