use super::*;
use yoctui_model::{SavedBuildAction, SavedEnvironmentAction, SavedEnvironmentRequest};

impl InteractiveRuntime {
    pub(super) fn handle_saved_environment_input(&mut self, input: Input) {
        if let Some(action) = yoctui_app::saved_build_workspace_action(&self.app, input) {
            let effect = compatibility_workspace_action(&mut self.app, action);
            self.submit_saved_environment_effect(effect);
        }
    }

    pub(super) fn submit_saved_environment_effect(&mut self, effect: Option<Effect>) {
        let Some(Effect::SavedEnvironment(request)) = effect else {
            return;
        };
        let generation = match &request {
            SavedEnvironmentRequest::Prepare { generation, .. }
            | SavedEnvironmentRequest::Load { generation, .. } => *generation,
        };
        // Local metadata workers are also tied to the old environment. Wait for
        // them rather than dropping work or letting stale results cross environments.
        if self.environment_bound_work_active() {
            let error = "local work is still active; wait for it to finish before loading another environment".to_owned();
            let action = match request {
                SavedEnvironmentRequest::Prepare { .. } => SavedEnvironmentAction::Prepared {
                    generation,
                    result: Err(error),
                },
                SavedEnvironmentRequest::Load { .. } => SavedEnvironmentAction::Finished {
                    generation,
                    result: Err(error),
                },
            };
            let _ = update(
                &mut self.app,
                Action::SavedBuild(SavedBuildAction::Environment(action)),
            );
            return;
        }
        self.saved_environment_operation = Some(crate::saved_environment::start(request));
    }

    fn environment_bound_work_active(&self) -> bool {
        self.environment_operation.is_some()
            || self.clone_operation.is_some()
            || self.signature_operation.is_some()
            || self.package_operation.is_some()
            || self.image_artifact_operation.is_some()
            || self.rootfs_composition_operation.is_some()
            || self.global_content_search_operation.is_some()
            || self.recipe_inspection_operation.is_some()
            || self.devtool_status_operation.is_some()
            || self.platform_inspection_operation.is_some()
            || self.devtool_runner.is_some()
            || self.sdk_artifact_operation.is_some()
            || self.sdk_capability_operation.is_some()
            || self.sdk_operation.is_some()
            || self.qemu_operation.is_some()
            || self.wic_capability_operation.is_some()
            || self.wic_device_operation.is_some()
            || self.wic_operation.is_some()
            || self.test_coordinator.session.is_some()
            || self.test_coordinator.import.is_some()
            || self.test_coordinator.result.is_some()
            || self.security_coordinator.capability.is_some()
            || self.security_coordinator.report.is_some()
            || self.security_coordinator.mapper.is_some()
            || self.qa_coordinator.capability.is_some()
            || self.qa_coordinator.layer_capability.is_some()
            || self.qa_coordinator.report.is_some()
            || self.qa_coordinator.layer.is_some()
            || self.detached_terminal_operation.is_some()
            || self.maintenance_coordinator.operation_active()
    }

    pub(super) async fn poll_saved_environment(&mut self) {
        if !self
            .saved_environment_operation
            .as_ref()
            .is_some_and(|operation| operation.task.is_finished())
        {
            return;
        }
        let mut operation = self
            .saved_environment_operation
            .take()
            .expect("finished saved environment operation");
        let result = (&mut operation.task).await;
        if self.app.saved_builds.environment.generation != operation.generation {
            return;
        }
        let result = result.unwrap_or_else(|error| Err(error.into()));
        let action = match result {
            Ok(crate::saved_environment::Outcome::Prepared(plan)) => {
                SavedEnvironmentAction::Prepared {
                    generation: operation.generation,
                    result: Ok(plan),
                }
            }
            Ok(crate::saved_environment::Outcome::Loaded {
                profile,
                mut runtime,
                client_id,
            }) => {
                runtime.install_saved_environment(&mut self.app, client_id);
                self.daemon_runtime = Some(*runtime);
                self.daemon_attached = true;
                let result = self
                    .rebind_loaded_environment(&profile)
                    .map(|()| profile)
                    .map_err(|error| error.to_string());
                SavedEnvironmentAction::Finished {
                    generation: operation.generation,
                    result,
                }
            }
            Err(error) if operation.loading => SavedEnvironmentAction::Finished {
                generation: operation.generation,
                result: Err(format!("{error:#}")),
            },
            Err(error) => SavedEnvironmentAction::Prepared {
                generation: operation.generation,
                result: Err(format!("{error:#}")),
            },
        };
        let _ = update(
            &mut self.app,
            Action::SavedBuild(SavedBuildAction::Environment(action)),
        );
        self.render_scheduler.invalidate(RenderCause::State);
    }

    fn rebind_loaded_environment(
        &mut self,
        profile: &yoctui_model::BuildEnvironmentProfile,
    ) -> Result<()> {
        self.session_build_dir = profile.build_dir.clone();
        self.signature_adapter = SignatureAdapter::new(self.session_build_dir.clone());
        self.package_adapter = PackageDataAdapter::new(self.session_build_dir.clone());
        self.image_artifact_adapter = self
            .app
            .workspace
            .variables
            .get("DEPLOY_DIR_IMAGE")
            .map(PathBuf::from)
            .map(ImageArtifactAdapter::new);
        self.sdk_artifact_adapter = self
            .app
            .workspace
            .variables
            .get("SDK_DEPLOY")
            .map(PathBuf::from)
            .map(SdkArtifactAdapter::new);
        self.sdk_tool_adapter =
            sdk_tool_adapter_for_workspace(&self.app, &self.session_build_dir).ok();
        self.qemu_inspector = qemu_capability_inspector(&self.app);
        self.wic_inspector = wic_capability_inspector(&self.app);
        let paths = initialized_path_directories();
        self.test_coordinator = TestCliCoordinator::new(
            self.session_build_dir.clone(),
            paths.clone(),
            ptest_capability(&self.app),
        );
        self.security_coordinator =
            SecurityCliCoordinator::new(self.session_build_dir.clone(), paths.clone());
        self.qa_coordinator = QaCliCoordinator::new(self.session_build_dir.clone(), paths.clone());
        self.maintenance_coordinator =
            MaintenanceCliCoordinator::new(&self.app, &self.session_build_dir, paths)
                .map_err(anyhow::Error::msg)?;
        self.source_git_poller = source_git::SourceGitPoller::default();
        self.metadata_backend_authoritative = false;
        if let Some(root) = project_profile_root(&self.session_build_dir) {
            let action = match load_project_profile(&root) {
                Ok(Some(profile)) => Action::ProjectProfileLoaded(profile),
                Ok(None) => Action::ProjectProfileAbsent,
                Err(error) => Action::ProjectProfileLoadFailed(error.to_string()),
            };
            let _ = update(&mut self.app, action);
        }
        let previous = std::mem::replace(
            &mut self.backend,
            Box::new(ProcessBackend::new(self.session_build_dir.clone())),
        );
        tokio::spawn(async move {
            let mut previous = previous;
            let _ = previous.shutdown().await;
        });
        Ok(())
    }
}
