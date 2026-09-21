use super::*;

impl InteractiveRuntime {
    pub(super) async fn route_remaining_workspaces(
        &mut self,
        input: Input,
    ) -> Result<Option<KeyRouteOutcome>> {
        let runtime = self;
        if runtime.app.screen == yoctui_model::Screen::RawMode
            && (runtime.app.focus == yoctui_model::FocusTarget::Workspace
                || matches!(
                    runtime.app.raw_mode.view,
                    yoctui_model::RawModeView::Form | yoctui_model::RawModeView::Preview
                ))
        {
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
                    effect @ (Effect::StartRaw(_)
                    | Effect::CancelRaw(_)
                    | Effect::SetRawAttachment { .. }),
                ) = effect
                {
                    #[cfg(unix)]
                    if let Some(daemon_client) = runtime.daemon_runtime.as_mut() {
                        match daemon_client.route_effect(&runtime.app, &effect) {
                            Ok(client_runtime::RuntimeEffectRoute::Daemon(_)) => {}
                            Ok(client_runtime::RuntimeEffectRoute::ClientLocal) => {
                                runtime.app.notification =
                                    Some("Raw execution was not routed to the daemon.".into());
                            }
                            Err(error) => {
                                runtime.app.notification =
                                    Some(format!("Raw execution was not sent: {error}"));
                            }
                        }
                    } else {
                        runtime.app.notification =
                            Some("Raw execution requires an attached Yoctui daemon.".into());
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
            inspect_selected_config_variable(&mut runtime.app, runtime.backend.as_mut()).await;
        } else if runtime.app.screen == yoctui_model::Screen::Configuration
            && matches!(
                input,
                Input::Char('s') | Input::Char('c') | Input::Char('E')
            )
        {
            if let Some(action) = config_workspace_action(false, input) {
                let _ = compatibility_workspace_action(&mut runtime.app, action);
            }
        } else if runtime.app.screen == yoctui_model::Screen::Configuration
            && matches!(input, Input::Char('C') | Input::Char('U'))
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
                } else if let Some(effect @ Effect::Cancel) =
                    compatibility_workspace_action(&mut runtime.app, action)
                {
                    #[cfg(unix)]
                    if let Some(daemon_client) = runtime.daemon_runtime.as_mut() {
                        match daemon_client.route_effect(&runtime.app, &effect) {
                            Ok(client_runtime::RuntimeEffectRoute::Daemon(_)) => {
                                return Ok(Some(KeyRouteOutcome::ContinueLoop));
                            }
                            Ok(client_runtime::RuntimeEffectRoute::ClientLocal) => {}
                            Err(error) => {
                                runtime.app.notification =
                                    Some(format!("Daemon cancellation was not sent: {error}"));
                                return Ok(Some(KeyRouteOutcome::ContinueLoop));
                            }
                        }
                    }
                    if let Some(job_action) = runtime.build_jobs.request_cancellation() {
                        let _ = compatibility_workspace_action(&mut runtime.app, job_action);
                    }
                    if let Err(error) = runtime.backend.cancel_build().await {
                        for action in runtime
                            .build_jobs
                            .cancellation_failed(error.to_string(), SystemTime::now())
                        {
                            let _ = compatibility_workspace_action(&mut runtime.app, action);
                        }
                    }
                }
            } else {
                if let Some(effect) = compatibility_workspace_action(&mut runtime.app, action) {
                    if local_workspace_effect_route(&effect)
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
