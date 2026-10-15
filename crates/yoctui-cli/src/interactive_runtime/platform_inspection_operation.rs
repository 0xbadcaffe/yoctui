use super::*;

const PLATFORM_INSPECTION_TIMEOUT: Duration = Duration::from_secs(120);

#[derive(Clone, Copy)]
pub(super) enum PlatformInspectionRequest {
    Kernel,
    Firmware,
}

impl PlatformInspectionRequest {
    pub(super) fn from_effect(effect: &Effect) -> Option<Self> {
        match effect {
            Effect::InspectKernel => Some(Self::Kernel),
            Effect::InspectFirmware => Some(Self::Firmware),
            _ => None,
        }
    }

    pub(super) fn screen(self) -> Screen {
        match self {
            Self::Kernel => Screen::Kernel,
            Self::Firmware => Screen::Firmware,
        }
    }
}

pub(super) struct PlatformInspectionOperation {
    request: PlatformInspectionRequest,
    handle: tokio::task::JoinHandle<Action>,
    cancel: tokio::sync::oneshot::Sender<()>,
}

impl InteractiveRuntime {
    pub(super) fn begin_platform_inspection(&mut self, request: PlatformInspectionRequest) {
        if self.platform_inspection_operation.is_some() {
            self.fail_platform_inspection(request, "another platform inspection is running".into());
            return;
        }
        if self.configuration_io.active() || self.recipe_inspection_operation.is_some() {
            // Keep the original request; the poll loop retries after the
            // other short-lived connection has released BitBake.
            self.startup_platform_inspection = Some(request.screen());
            return;
        }
        let build_dir = self.session_build_dir.clone();
        let source_dir = self.app.workspace.source_dir.clone();
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
        let (cancel, mut cancelled) = tokio::sync::oneshot::channel();
        let handle = tokio::spawn(async move {
            if let Err(message) =
                super::metadata_backend::metadata_backend_start_required(&backend_kind, false)
            {
                return failed_action(request, message.into());
            }
            let mut environment = match tokio::select! {
                _ = &mut cancelled => return failed_action(request, "inspection cancelled".into()),
                result = initialized_platform_environment(&build_dir, source_dir.as_deref()) => result,
            } {
                Ok(environment) => environment,
                Err(error) => return failed_action(request, format!("{error:#}")),
            };
            environment.insert("YOCTUI_BRIDGE_FAST_RECIPE_INSPECTION".into(), "1".into());
            let python = env::var("PYTHON").unwrap_or_else(|_| "python3".into());
            let mut backend = match tokio::select! {
                _ = &mut cancelled => return failed_action(request, "inspection cancelled".into()),
                result = spawn_configured_bridge(&python, build_dir, Some(environment)) => result,
            } {
                Ok(backend) => backend.with_platform_inspection_scope(),
                Err(error) => return failed_action(request, format!("{error:#}")),
            };
            let inspection = async {
                match request {
                    PlatformInspectionRequest::Kernel => {
                        inspect_kernel_workbench(deploy_dir, &mut backend).await
                    }
                    PlatformInspectionRequest::Firmware => {
                        inspect_firmware_workbench(image, recipes, deploy_dir, &mut backend).await
                    }
                }
            };
            let (action, completed) = tokio::select! {
                _ = &mut cancelled => (failed_action(request, "inspection cancelled".into()), false),
                result = tokio::time::timeout(PLATFORM_INSPECTION_TIMEOUT, inspection) => match result {
                    Ok(action) => (action, true),
                    Err(_) => (failed_action(request, "platform inspection timed out after 120 seconds".into()), false),
                },
            };
            let released = completed
                && tokio::time::timeout(Duration::from_secs(5), backend.shutdown())
                    .await
                    .is_ok_and(|result| result.is_ok());
            if !released {
                #[cfg(unix)]
                backend.interrupt_metadata().await;
            }
            action
        });
        self.platform_inspection_operation = Some(PlatformInspectionOperation {
            request,
            handle,
            cancel,
        });
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
            let _ = operation.cancel.send(());
            let _ = operation.handle.await;
        }
    }
}

pub(crate) async fn initialized_platform_environment(
    build_dir: &Path,
    source_dir: Option<&Path>,
) -> Result<BTreeMap<String, String>> {
    let profile = match source_dir {
        Some(source) => daemon_commands::selected_build_environment_profile(build_dir, source),
        None => daemon_commands::inferred_build_environment_profile(build_dir),
    }
    .with_context(|| {
        format!(
            "cannot locate oe-init-build-env for the selected build directory {}",
            build_dir.display()
        )
    })?;
    let response = BuildEnvironmentAdapter::default()
        .initialize(profile)
        .await
        .context("could not initialize the selected build environment for platform inspection")?;
    let initialized_build = response
        .environment
        .get("BUILDDIR")
        .map(PathBuf::from)
        .and_then(|path| path.canonicalize().ok());
    anyhow::ensure!(
        initialized_build.as_ref() == Some(&response.profile.build_dir),
        "platform inspection initializer selected a different BUILDDIR"
    );
    Ok(response.environment)
}

fn failed_action(request: PlatformInspectionRequest, message: String) -> Action {
    match request {
        PlatformInspectionRequest::Kernel => Action::KernelFailed(message),
        PlatformInspectionRequest::Firmware => Action::FirmwareFailed(message),
    }
}

#[cfg(test)]
#[path = "../tests/platform_inspection_environment.rs"]
mod tests;
