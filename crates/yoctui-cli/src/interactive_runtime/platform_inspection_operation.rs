use super::*;

const PLATFORM_INSPECTION_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone, Copy)]
pub(super) enum PlatformInspectionRequest {
    Kernel,
    Firmware,
}

pub(super) struct PlatformInspectionOperation {
    request: PlatformInspectionRequest,
    handle: tokio::task::JoinHandle<Action>,
}

impl InteractiveRuntime {
    pub(super) fn begin_platform_inspection(&mut self, request: PlatformInspectionRequest) {
        if self.platform_inspection_operation.is_some() {
            self.fail_platform_inspection(request, "another platform inspection is running".into());
            return;
        }
        let build_dir = self.session_build_dir.clone();
        let cancellation_timeout = self.cancellation_timeout;
        let backend_kind = self.backend_kind.clone();
        let deploy_dir = self
            .app
            .workspace
            .variables
            .get("DEPLOY_DIR_IMAGE")
            .map(PathBuf::from);
        let image = self.app.build.target.clone();
        let recipes = self
            .app
            .workspace
            .recipes
            .iter()
            .map(|recipe| recipe.name.clone())
            .collect();
        let handle = tokio::spawn(async move {
            if let Err(message) =
                super::metadata_backend::metadata_backend_start_required(&backend_kind, false)
            {
                return failed_action(request, message.into());
            }
            let mut backend = match select_backend_with_timeout(
                Backend::Bridge,
                build_dir,
                Some(cancellation_timeout),
            )
            .await
            {
                Ok(backend) => backend,
                Err(error) => return failed_action(request, format!("{error:#}")),
            };
            let inspection = async {
                match request {
                    PlatformInspectionRequest::Kernel => {
                        inspect_kernel_workbench(deploy_dir, backend.as_mut()).await
                    }
                    PlatformInspectionRequest::Firmware => {
                        inspect_firmware_workbench(image, recipes, deploy_dir, backend.as_mut())
                            .await
                    }
                }
            };
            let action = match tokio::time::timeout(PLATFORM_INSPECTION_TIMEOUT, inspection).await {
                Ok(action) => action,
                Err(_) => failed_action(
                    request,
                    "platform inspection timed out after 120 seconds".into(),
                ),
            };
            if let Err(error) = backend.shutdown().await {
                tracing::debug!(%error, "platform metadata backend shutdown failed");
            }
            action
        });
        self.platform_inspection_operation = Some(PlatformInspectionOperation { request, handle });
    }

    pub(super) async fn poll_platform_inspection(&mut self) -> bool {
        if !self
            .platform_inspection_operation
            .as_ref()
            .is_some_and(|operation| operation.handle.is_finished())
        {
            return false;
        }
        let operation = self
            .platform_inspection_operation
            .take()
            .expect("finished platform inspection operation");
        let action = match operation.handle.await {
            Ok(action) => action,
            Err(error) => failed_action(
                operation.request,
                format!("platform inspection task failed: {error}"),
            ),
        };
        let _ = compatibility_workspace_action(&mut self.app, action);
        true
    }

    fn fail_platform_inspection(&mut self, request: PlatformInspectionRequest, message: String) {
        let _ = compatibility_workspace_action(&mut self.app, failed_action(request, message));
    }

    pub(super) async fn stop_platform_inspection(&mut self) {
        if let Some(operation) = self.platform_inspection_operation.take() {
            operation.handle.abort();
            let _ = operation.handle.await;
        }
    }
}

fn failed_action(request: PlatformInspectionRequest, message: String) -> Action {
    match request {
        PlatformInspectionRequest::Kernel => Action::KernelFailed(message),
        PlatformInspectionRequest::Firmware => Action::FirmwareFailed(message),
    }
}
