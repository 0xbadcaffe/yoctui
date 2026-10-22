use super::*;

impl InteractiveRuntime {
    pub(super) async fn route_remaining_workspaces(
        &mut self,
        input: Input,
    ) -> Result<Option<KeyRouteOutcome>> {
        let runtime = self;
        if yoctui_app::raw_workspace_owns_input(&runtime.app, input) {
            if let Some(action) = raw_mode_input(&runtime.app, input) {
                let effect =
                    compatibility_workspace_action(&mut runtime.app, Action::RawMode(action));
                if matches!(effect.as_ref(), Some(Effect::PersistSettings)) {
                    let favorites = runtime.app.raw_mode.favorites.clone();
                    if let Err(error) = persist_raw_favorites(
                        runtime.session_path.as_deref(),
                        &mut runtime.session,
                        &favorites,
                    ) {
                        runtime.app.notification =
                            Some(format!("Raw favorites could not be saved: {error}"));
                    }
                }
                if let Some(
                    effect @ (Effect::Terminal(_)
                    | Effect::StartRaw(_)
                    | Effect::CancelRaw(_)
                    | Effect::SetRawAttachment { .. }),
                ) = effect
                {
                    #[cfg(unix)]
                    let failure = if let Some(daemon_client) = runtime.daemon_runtime.as_mut() {
                        match daemon_client.route_effect(&runtime.app, &effect) {
                            Ok(client_runtime::RuntimeEffectRoute::Daemon(_)) => None,
                            Ok(client_runtime::RuntimeEffectRoute::ClientLocal) => {
                                Some("Raw execution was not routed to the daemon.".into())
                            }
                            Err(error) => Some(format!("Raw execution was not sent: {error}")),
                        }
                    } else {
                        Some("Raw execution requires an attached Yoctui daemon.".into())
                    };
                    #[cfg(unix)]
                    if let Some(message) = failure {
                        if let Effect::StartRaw(request) = &effect {
                            compatibility_workspace_action(
                                &mut runtime.app,
                                Action::RawMode(yoctui_model::RawModeAction::SubmissionFailed {
                                    request: request.id.clone(),
                                    message: message.clone(),
                                }),
                            );
                        }
                        runtime.app.notification = Some(message);
                    }
                }
            }
        } else if runtime.app.screen == yoctui_model::Screen::Compatibility {
            if let Some(action) =
                compatibility_ui_inspector_action(runtime.app.compatibility_ui.searching, input)
            {
                let _ = compatibility_workspace_action(&mut runtime.app, action);
            }
        } else if runtime.app.screen == yoctui_model::Screen::Configuration
            && collection_scroll_delta(input).is_some()
        {
            if let Some(action) = config_workspace_action(false, input) {
                let _ = compatibility_workspace_action(&mut runtime.app, action);
            }
        } else if runtime.app.screen == yoctui_model::Screen::Configuration && input == Input::Enter
        {
            if let Some(Effect::GetVariable(identity)) =
                compatibility_workspace_action(&mut runtime.app, Action::BeginSelectedConfigDetail)
            {
                runtime.configuration_io.inspect(identity);
            }
        } else if runtime.app.screen == yoctui_model::Screen::Configuration
            && matches!(input, Input::Char('r') | Input::Alt('r') | Input::Char('R'))
        {
            runtime.configuration_io.refresh();
        } else if runtime.app.screen == yoctui_model::Screen::Configuration
            && matches!(
                input,
                Input::Char('s') | Input::Char('c') | Input::Alt('e') | Input::Char('E')
            )
        {
            if let Some(action) = config_workspace_action(false, input) {
                let _ = compatibility_workspace_action(&mut runtime.app, action);
            }
        } else if runtime.app.screen == yoctui_model::Screen::Configuration
            && matches!(
                input,
                Input::Alt('c') | Input::Char('C') | Input::Alt('u') | Input::Char('U')
            )
        {
            if let Some(Effect::CopyToClipboard(content)) =
                config_copy_effect(&mut runtime.app, input)
            {
                copy_to_clipboard(&mut runtime.app, content).await;
            }
        } else if runtime.app.screen == yoctui_model::Screen::Configuration
            && input == Input::Char('o')
        {
            if let Some(Effect::OpenInEditor(path)) =
                compatibility_workspace_action(&mut runtime.app, Action::OpenSelectedConfigSource)
            {
                open_in_editor(
                    &runtime.guard,
                    &mut runtime.app,
                    path,
                    runtime.editor_command.as_deref(),
                )
                .await;
            }
        } else if runtime.app.screen == yoctui_model::Screen::Bbmask && input == Input::Char('e') {
            let _ = compatibility_workspace_action(&mut runtime.app, Action::BeginBbmaskEdit);
        } else if matches!(
            runtime.app.screen,
            yoctui_model::Screen::Recipes
                | yoctui_model::Screen::Layers
                | yoctui_model::Screen::Configuration
        ) && input == Input::Char('/')
        {
            let _ = compatibility_workspace_action(&mut runtime.app, Action::BeginMetadataSearch);
        } else if runtime.app.logs.searching {
            match input {
                Input::Char(character) => {
                    let _ = compatibility_workspace_action(
                        &mut runtime.app,
                        Action::AppendLogQuery(character),
                    );
                }
                Input::Enter | Input::Esc => {
                    let _ =
                        compatibility_workspace_action(&mut runtime.app, Action::FinishLogSearch);
                }
                Input::Backspace => {
                    let _ =
                        compatibility_workspace_action(&mut runtime.app, Action::BackspaceLogQuery);
                }
                _ => {}
            }
        } else if let Some(action) = keymap_action_for_app(&mut runtime.app, input).action() {
            if matches!(action, Action::Cancel) {
                if runtime.devtool_jobs.active_job_id().is_some() {
                    if let Some(job_action) = runtime.devtool_jobs.request_cancellation() {
                        let _ = compatibility_workspace_action(&mut runtime.app, job_action);
                    }
                    let cancellation = if let Some(runner) = runtime.devtool_runner.as_mut() {
                        runner.cancel().await.map(|_| ())
                    } else {
                        Err(yoctui_bitbake::DevtoolRunnerError::NotRunning)
                    };
                    if let Err(error) = cancellation {
                        for action in runtime
                            .devtool_jobs
                            .cancellation_failed(error.to_string(), SystemTime::now())
                        {
                            let _ = compatibility_workspace_action(&mut runtime.app, action);
                        }
                    }
                } else if let Some(Effect::Cancel) =
                    compatibility_workspace_action(&mut runtime.app, action)
                {
                    runtime.cancel_build().await;
                }
            } else {
                if let Some(effect) = compatibility_workspace_action(&mut runtime.app, action) {
                    if let Effect::LoadLayerBrowserDirectory {
                        layer,
                        root,
                        directory,
                    } = effect
                    {
                        runtime.layer_browser_io.directory(
                            &mut runtime.app,
                            layer,
                            root,
                            directory,
                        );
                    } else if local_workspace_effect_route(&effect)
                        == LocalWorkspaceEffectRoute::ImageArtifacts
                    {
                        begin_image_artifact_operation(
                            &mut runtime.app,
                            runtime.image_artifact_adapter.as_ref(),
                            &mut runtime.image_artifact_operation,
                            effect,
                        );
                    } else if local_workspace_effect_route(&effect)
                        == LocalWorkspaceEffectRoute::RootfsComposition
                    {
                        begin_rootfs_composition_operation(
                            runtime.backend.as_mut(),
                            &mut runtime.app,
                            &runtime.session_build_dir,
                            &mut runtime.rootfs_composition_operation,
                            effect,
                            runtime.daemon_attached,
                        )
                        .await;
                    } else if !route_independent_security_effect(
                        &runtime.guard,
                        &mut runtime.app,
                        &mut runtime.security_coordinator,
                        effect.clone(),
                        runtime.editor_command.as_deref(),
                    )
                    .await
                        && !route_independent_qa_effect(
                            &runtime.guard,
                            &mut runtime.app,
                            &mut runtime.qa_coordinator,
                            effect.clone(),
                            runtime.editor_command.as_deref(),
                        )
                        .await
                        && !route_independent_maintenance_effect(
                            &runtime.guard,
                            &mut runtime.app,
                            &mut runtime.maintenance_coordinator,
                            effect.clone(),
                            runtime.editor_command.as_deref(),
                        )
                        .await
                    {
                        let _ = runtime
                            .test_coordinator
                            .handle_effect(&mut runtime.app, effect)
                            .await;
                    }
                }
            }
        } else {
            return Ok(None);
        }
        Ok(Some(KeyRouteOutcome::Handled))
    }
}
