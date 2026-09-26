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
            let environment = match initialized_platform_environment(&build_dir).await {
                Ok(environment) => environment,
                Err(error) => return failed_action(request, format!("{error:#}")),
            };
            let mut backend = match select_backend_with_environment(
                Backend::Bridge,
                build_dir,
                Some(cancellation_timeout),
                Some(environment),
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

async fn initialized_platform_environment(build_dir: &Path) -> Result<BTreeMap<String, String>> {
    let profile =
        daemon_commands::inferred_build_environment_profile(build_dir).with_context(|| {
            format!(
                "cannot locate oe-init-build-env for the selected build directory {}",
                build_dir.display()
            )
        })?;
    let response = BuildEnvironmentAdapter::default()
        .initialize(profile)
        .await
        .context("could not initialize the selected build environment for platform inspection")?;
    Ok(response.environment)
}

fn failed_action(request: PlatformInspectionRequest, message: String) -> Action {
    match request {
        PlatformInspectionRequest::Kernel => Action::KernelFailed(message),
        PlatformInspectionRequest::Firmware => Action::FirmwareFailed(message),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn platform_inspection_reconstructs_the_selected_build_environment() {
        use std::os::unix::fs::PermissionsExt;

        let root = std::env::temp_dir().join(format!(
            "yoctui-platform-inspection-environment-{}-{}",
            std::process::id(),
            yoctui_utils::unix_ms()
        ));
        let build_dir = root.join("build/romulus");
        fs::create_dir_all(build_dir.join("conf")).unwrap();
        fs::write(build_dir.join("conf/local.conf"), "MACHINE = \"romulus\"\n").unwrap();
        fs::write(build_dir.join("conf/bblayers.conf"), "BBLAYERS = \"\"\n").unwrap();
        let init_script = root.join("oe-init-build-env");
        fs::write(
            &init_script,
            "#!/usr/bin/env bash\nexport BUILDDIR=\"$1\"\nexport YOCTUI_PLATFORM_INSPECTION_TEST=ready\n",
        )
        .unwrap();
        let mut permissions = fs::metadata(&init_script).unwrap().permissions();
        permissions.set_mode(0o700);
        fs::set_permissions(&init_script, permissions).unwrap();

        let environment = initialized_platform_environment(&build_dir).await.unwrap();

        assert_eq!(
            environment.get("YOCTUI_PLATFORM_INSPECTION_TEST"),
            Some(&"ready".to_owned())
        );
        assert_eq!(
            environment.get("BUILDDIR"),
            Some(&build_dir.display().to_string())
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[tokio::test]
    async fn platform_inspection_reports_a_missing_selected_profile() {
        let missing = std::env::temp_dir().join(format!(
            "yoctui-platform-inspection-missing-{}-{}",
            std::process::id(),
            yoctui_utils::unix_ms()
        ));

        let error = initialized_platform_environment(&missing)
            .await
            .expect_err("missing build profile must fail");

        assert!(
            error
                .to_string()
                .contains("cannot locate oe-init-build-env")
        );
    }
}
