use super::*;

impl InteractiveRuntime {
    pub(super) async fn poll_jobs(&mut self) {
        let runtime = self;
        let devtool_was_active = runtime.devtool_runner.is_some();
        let completed_devtool = poll_devtool_job(
            &mut runtime.app,
            &mut runtime.devtool_jobs,
            &mut runtime.devtool_runner,
        )
        .await;
        runtime
            .render_scheduler
            .invalidate_if(devtool_was_active, RenderCause::Presentation);
        match completed_devtool {
            Some(DevtoolOperation::Modify { recipe })
                if runtime
                    .pending_devtool_modify
                    .as_ref()
                    .is_some_and(|identity| identity.name == recipe) =>
            {
                if let Some(identity) = runtime.pending_devtool_modify.take() {
                    complete_devtool_modify(&mut runtime.app, &runtime.session_build_dir, identity)
                        .await;
                }
            }
            Some(DevtoolOperation::UpdateRecipe { recipe })
                if runtime
                    .pending_devtool_update
                    .as_ref()
                    .is_some_and(|identity| identity.name == recipe) =>
            {
                if let Some(identity) = runtime.pending_devtool_update.take() {
                    complete_devtool_update(&mut runtime.app, &runtime.session_build_dir, identity)
                        .await;
                }
            }
            Some(DevtoolOperation::Finish { recipe, .. })
                if runtime
                    .pending_devtool_finish
                    .as_ref()
                    .is_some_and(|identity| identity.name == recipe) =>
            {
                if let Some(identity) = runtime.pending_devtool_finish.take() {
                    complete_devtool_finish(&mut runtime.app, &runtime.session_build_dir, identity)
                        .await;
                }
            }
            Some(DevtoolOperation::DeployTarget { recipe, .. })
                if runtime
                    .pending_devtool_deploy
                    .as_ref()
                    .is_some_and(|identity| identity.name == recipe) =>
            {
                if let Some(identity) = runtime.pending_devtool_deploy.take() {
                    complete_devtool_deploy(&mut runtime.app, &runtime.session_build_dir, identity)
                        .await;
                }
            }
            Some(DevtoolOperation::UndeployTarget { recipe, .. })
                if runtime
                    .pending_devtool_undeploy
                    .as_ref()
                    .is_some_and(|identity| identity.name == recipe) =>
            {
                if let Some(identity) = runtime.pending_devtool_undeploy.take() {
                    complete_devtool_undeploy(
                        &mut runtime.app,
                        &runtime.session_build_dir,
                        identity,
                    )
                    .await;
                }
            }
            Some(DevtoolOperation::Upgrade { recipe })
                if runtime
                    .pending_devtool_upgrade
                    .as_ref()
                    .is_some_and(|identity| identity.name == recipe) =>
            {
                if let Some(identity) = runtime.pending_devtool_upgrade.take() {
                    complete_devtool_upgrade(
                        &mut runtime.app,
                        &runtime.session_build_dir,
                        identity,
                    )
                    .await;
                }
            }
            Some(DevtoolOperation::Reset { recipe })
                if runtime
                    .pending_devtool_reset
                    .as_ref()
                    .is_some_and(|identity| identity.name == recipe) =>
            {
                if let Some(identity) = runtime.pending_devtool_reset.take() {
                    complete_devtool_reset(&mut runtime.app, &runtime.session_build_dir, identity)
                        .await;
                }
            }
            _ if runtime.devtool_jobs.active_operation().is_none() => {
                runtime.pending_devtool_modify = None;
                runtime.pending_devtool_update = None;
                runtime.pending_devtool_finish = None;
                runtime.pending_devtool_deploy = None;
                runtime.pending_devtool_undeploy = None;
                runtime.pending_devtool_upgrade = None;
                runtime.pending_devtool_reset = None;
            }
            _ => {}
        }
        if runtime.build_jobs.active_job_id().is_some() {
            match tokio::time::timeout(Duration::from_millis(1), runtime.backend.next_event()).await
            {
                Ok(Ok(event)) => {
                    runtime.render_scheduler.invalidate(RenderCause::State);
                    let test_terminal = matches!(
                        event,
                        BackendEvent::BuildCompleted { .. }
                            | BackendEvent::CommandFailed { .. }
                            | BackendEvent::Disconnected
                    );
                    let security_terminal = test_terminal;
                    let qa_terminal = test_terminal;
                    if let Some(id) = runtime.pending_test_build
                        && let Some(action) = test_build_action_for_event(&runtime.app, id, &event)
                    {
                        let _ = compatibility_workspace_action(&mut runtime.app, action);
                    }
                    let security_followup = runtime
                        .pending_security_build
                        .and_then(|id| security_build_action_for_event(&runtime.app, id, &event))
                        .and_then(|action| {
                            compatibility_workspace_action(&mut runtime.app, action)
                        });
                    let qa_followup = runtime
                        .pending_qa_build
                        .and_then(|id| qa_build_action_for_event(&runtime.app, id, &event))
                        .and_then(|action| {
                            compatibility_workspace_action(&mut runtime.app, action)
                        });
                    let sdk_refresh = sdk_refresh_after_build_event(
                        &mut runtime.app,
                        &mut runtime.pending_sdk_build,
                        &event,
                    );
                    for action in runtime
                        .build_jobs
                        .actions_for_backend_event(event, SystemTime::now())
                    {
                        let _ = compatibility_workspace_action(&mut runtime.app, action);
                    }
                    if test_terminal {
                        runtime.pending_test_build = None;
                    }
                    if security_terminal {
                        runtime.pending_security_build = None;
                    }
                    if qa_terminal {
                        runtime.pending_qa_build = None;
                    }
                    if let Some(effect) = security_followup {
                        let _ = runtime
                            .security_coordinator
                            .handle_effect(&mut runtime.app, effect)
                            .await;
                    }
                    if let Some(effect) = qa_followup {
                        let _ = runtime
                            .qa_coordinator
                            .handle_effect(&mut runtime.app, effect)
                            .await;
                    }
                    if let Some(effect) = sdk_refresh {
                        begin_sdk_artifact_operation(
                            &mut runtime.app,
                            runtime.sdk_artifact_adapter.as_ref(),
                            &mut runtime.sdk_artifact_operation,
                            effect,
                        );
                    }
                }
                Ok(Err(error)) => {
                    runtime.render_scheduler.invalidate(RenderCause::State);
                    if let Some(id) = runtime.pending_test_build.take() {
                        let _ = update(
                            &mut runtime.app,
                            Action::LoseTestSession {
                                id,
                                message: error.to_string(),
                                finished_at: SystemTime::now(),
                            },
                        );
                    }
                    if let Some(id) = runtime.pending_security_build.take() {
                        let _ = update(
                            &mut runtime.app,
                            Action::Security(SecurityAction::LoseSession {
                                id,
                                message: error.to_string(),
                                finished_at: SystemTime::now(),
                            }),
                        );
                    }
                    if let Some(id) = runtime.pending_qa_build.take() {
                        let _ = update(
                            &mut runtime.app,
                            Action::Qa(QaAction::LoseSession {
                                session: id,
                                message: error.to_string(),
                                finished_at: SystemTime::now(),
                            }),
                        );
                    }
                    runtime.pending_sdk_build = None;
                    for action in runtime
                        .build_jobs
                        .backend_lost(error.to_string(), SystemTime::now())
                    {
                        let _ = compatibility_workspace_action(&mut runtime.app, action);
                    }
                }
                Err(_) => {}
            }
        }
    }
}
