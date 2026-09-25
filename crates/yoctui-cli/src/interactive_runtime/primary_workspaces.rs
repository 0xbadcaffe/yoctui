use super::dependency_workspace::route_dependency_workspace;
use super::*;

impl InteractiveRuntime {
    pub(super) async fn route_terminal_and_primary_workspaces(
        &mut self,
        input: Input,
        replayed_context_action: bool,
    ) -> Result<Option<KeyRouteOutcome>> {
        let runtime = self;
        if runtime.app.screen == Screen::TerminalSessions
            || runtime.app.platform_menuconfig_visible()
        {
            let terminal_action = if replayed_context_action {
                yoctui_app::terminal_context_action(input)
            } else {
                yoctui_app::terminal_workspace_action(&runtime.app, input)
            };
            if let Some(action) = terminal_action {
                match compatibility_workspace_action(&mut runtime.app, action) {
                    Some(effect @ Effect::Terminal(_)) => {
                        let _ = submit_daemon_effect(
                            &mut runtime.daemon_runtime,
                            &mut runtime.app,
                            &effect,
                        );
                    }
                    Some(Effect::CopyToClipboard(content)) => {
                        copy_to_clipboard(&mut runtime.app, content).await;
                    }
                    _ => {}
                }
            } else if runtime.app.selected_terminal_is_writer() {
                if let (Some(bytes), Some(session), Some(details)) = (
                    terminal_input_bytes(input),
                    runtime.app.selected_terminal_session(),
                    runtime.app.selected_terminal_details(),
                ) {
                    let effect = Effect::Terminal(yoctui_model::TerminalEffect::Input {
                        session_id: session.id,
                        writer_epoch: details.writer_epoch,
                        bytes,
                    });
                    let _ = submit_daemon_effect(
                        &mut runtime.daemon_runtime,
                        &mut runtime.app,
                        &effect,
                    );
                }
            } else {
                runtime.app.notification = Some(
                    "Terminal is read-only; press o or Ctrl+B o to take writer control.".into(),
                );
            }
        } else if let Some(action) = notification_input_action(
            runtime.app.notification.is_some(),
            runtime.app.build.status == BuildStatus::Failed
                && runtime.app.logs.diagnostics().next().is_some(),
            runtime.app.screen == Screen::Settings && runtime.app.settings_dirty,
            input,
        ) {
            let _ = compatibility_workspace_action(&mut runtime.app, action);
        } else if matches!(runtime.app.screen, Screen::Kernel | Screen::Firmware)
            && let Some(action) = match runtime.app.screen {
                Screen::Kernel => platform_workspace_action(input),
                Screen::Firmware => firmware_workspace_action(input),
                _ => None,
            }
        {
            match compatibility_workspace_action(&mut runtime.app, action) {
                Some(Effect::InspectKernel) => {
                    runtime.begin_platform_inspection(
                        platform_inspection_operation::PlatformInspectionRequest::Kernel,
                    );
                }
                Some(Effect::InspectFirmware) => {
                    runtime.begin_platform_inspection(
                        platform_inspection_operation::PlatformInspectionRequest::Firmware,
                    );
                }
                Some(Effect::OpenWorkspaceEditor { label, root }) => {
                    open_workspace_editor(&mut runtime.app, label, root).await;
                }
                Some(Effect::OpenLayerBrowserEditor { layer, root, file }) => {
                    if let Some(Effect::LoadRecipeEditorFile(path)) = compatibility_workspace_action(
                        &mut runtime.app,
                        Action::OpenRecipeEditor {
                            recipe: layer,
                            root,
                            files: vec![file],
                        },
                    ) {
                        load_recipe_editor_file(&mut runtime.app, path).await;
                    }
                }
                _ => {}
            }
        } else if runtime.app.screen == Screen::Insights
            && let Some(action) = overview_workspace_action(input)
        {
            let _ = compatibility_workspace_action(&mut runtime.app, action);
        } else if runtime.app.screen != Screen::Layers
            && collection_scroll_delta(input).is_some()
            && let Some(action) = workspace_collection_action(&runtime.app, input)
        {
            let _ = compatibility_workspace_action(&mut runtime.app, action);
        } else if runtime.app.screen == Screen::Packages
            && package_workspace_action(runtime.app.package_searching, input).is_some()
        {
            let action = package_workspace_action(runtime.app.package_searching, input)
                .expect("Packages action was checked");
            match compatibility_workspace_action(&mut runtime.app, action) {
                Some(effect @ (Effect::GetPackageInventory(_) | Effect::GetPackageDetail(_))) => {
                    begin_package_operation(
                        &mut runtime.app,
                        &runtime.package_adapter,
                        &mut runtime.package_operation,
                        effect,
                    )
                }
                Some(Effect::CancelPackageOperation) => {
                    if let Some(operation) = runtime.package_operation.as_ref() {
                        if operation.cancellation.cancel() {
                            runtime.app.notification =
                                Some("Package-data cancellation requested.".into());
                        }
                    } else {
                        runtime.app.notification =
                            Some("No package-data operation is running.".into());
                    }
                }
                Some(Effect::OpenInEditor(path)) => {
                    open_in_editor(
                        &runtime.guard,
                        &mut runtime.app,
                        path,
                        runtime.editor_command.as_deref(),
                    )
                    .await;
                }
                _ => {}
            }
        } else if runtime.app.screen == Screen::Images
            && images_workspace_action_for_view(
                runtime.app.image_artifact_searching,
                runtime.app.images_view,
                input,
            )
            .is_some()
        {
            let action = images_workspace_action_for_view(
                runtime.app.image_artifact_searching,
                runtime.app.images_view,
                input,
            )
            .expect("Images action was checked");
            match compatibility_workspace_action(&mut runtime.app, action) {
                Some(effect @ Effect::GetImageArtifacts(_)) => begin_image_artifact_operation(
                    &mut runtime.app,
                    runtime.image_artifact_adapter.as_ref(),
                    &mut runtime.image_artifact_operation,
                    effect,
                ),
                Some(effect @ Effect::GetRootfsComposition(_)) => {
                    begin_rootfs_composition_operation(
                        runtime.backend.as_mut(),
                        &mut runtime.app,
                        &runtime.session_build_dir,
                        &mut runtime.rootfs_composition_operation,
                        effect,
                        runtime.daemon_attached,
                    )
                    .await
                }
                Some(effect @ Effect::GetWicDevices(_)) => begin_wic_device_operation(
                    &runtime.wic_device_inspector,
                    &mut runtime.wic_device_operation,
                    effect,
                ),
                Some(Effect::CancelImageArtifactOperation) => {
                    if let Some(operation) = runtime.image_artifact_operation.as_ref() {
                        if operation.cancellation.cancel() {
                            runtime.app.notification =
                                Some("Image artifact cancellation requested.".into());
                        }
                    } else {
                        runtime.app.notification =
                            Some("No image artifact operation is running.".into());
                    }
                }
                Some(Effect::OpenInEditor(path)) => {
                    open_in_editor(
                        &runtime.guard,
                        &mut runtime.app,
                        path,
                        runtime.editor_command.as_deref(),
                    )
                    .await;
                }
                Some(Effect::OpenLayerBrowserEditor { layer, root, file }) => {
                    if let Some(Effect::LoadRecipeEditorFile(path)) = compatibility_workspace_action(
                        &mut runtime.app,
                        Action::OpenRecipeEditor {
                            recipe: layer,
                            root,
                            files: vec![file],
                        },
                    ) {
                        load_recipe_editor_file(&mut runtime.app, path).await;
                    }
                }
                Some(Effect::LoadLayerBrowserDirectory {
                    layer,
                    root,
                    directory,
                }) => {
                    load_layer_browser_directory(&mut runtime.app, layer, root, directory).await;
                }
                _ => {}
            }
        } else if runtime.app.screen == Screen::Sdk
            && sdk_workspace_action(runtime.app.sdk_artifact_searching, input).is_some()
        {
            let action = sdk_workspace_action(runtime.app.sdk_artifact_searching, input)
                .expect("SDK action was checked");
            match compatibility_workspace_action(&mut runtime.app, action) {
                Some(effect @ Effect::GetSdkArtifacts(_)) => begin_sdk_artifact_operation(
                    &mut runtime.app,
                    runtime.sdk_artifact_adapter.as_ref(),
                    &mut runtime.sdk_artifact_operation,
                    effect,
                ),
                Some(Effect::CancelSdkArtifactOperation) => {
                    if let Some(operation) = runtime.sdk_artifact_operation.as_ref() {
                        if operation.cancellation.cancel() {
                            runtime.app.notification =
                                Some("SDK artifact cancellation requested.".into());
                        }
                    } else {
                        runtime.app.notification = Some("No SDK artifact scan is running.".into());
                    }
                }
                Some(effect @ Effect::InspectSdkTools) => begin_sdk_capability_operation(
                    &mut runtime.app,
                    runtime.sdk_tool_adapter.as_ref(),
                    &mut runtime.sdk_capability_operation,
                    effect,
                ),
                Some(Effect::OpenInEditor(path)) => {
                    open_in_editor(
                        &runtime.guard,
                        &mut runtime.app,
                        path,
                        runtime.editor_command.as_deref(),
                    )
                    .await;
                }
                _ => {}
            }
        } else if runtime.app.screen == Screen::Testing
            && testing_screen_action(&runtime.app, input).is_some()
        {
            let action =
                testing_screen_action(&runtime.app, input).expect("Testing action was checked");
            match compatibility_workspace_action(&mut runtime.app, action) {
                Some(effect @ Effect::ImportTestResults(_))
                | Some(effect @ Effect::CompareTestResults(_)) => {
                    if submit_daemon_effect(&mut runtime.daemon_runtime, &mut runtime.app, &effect)
                        .is_none()
                    {
                        let _ = runtime
                            .test_coordinator
                            .handle_effect(&mut runtime.app, effect)
                            .await;
                    }
                }
                Some(effect @ Effect::InspectTestJunitDestination { .. })
                | Some(effect @ Effect::ExportTestJunit(_))
                | Some(effect @ Effect::InspectTestCapability)
                | Some(effect @ Effect::InspectResultToolCapability) => {
                    let _ = runtime
                        .test_coordinator
                        .handle_effect(&mut runtime.app, effect)
                        .await;
                }
                Some(Effect::OpenInEditor(path)) => {
                    open_in_editor(
                        &runtime.guard,
                        &mut runtime.app,
                        path,
                        runtime.editor_command.as_deref(),
                    )
                    .await;
                }
                _ => {}
            }
        } else if runtime.app.screen == Screen::Security
            && security_workspace_action(
                runtime.app.security.view,
                runtime.app.security.drilled,
                runtime.app.security.searching,
                input,
            )
            .is_some()
        {
            let action = security_workspace_action(
                runtime.app.security.view,
                runtime.app.security.drilled,
                runtime.app.security.searching,
                input,
            )
            .expect("Security action was checked");
            if let Some(effect) = compatibility_workspace_action(&mut runtime.app, action) {
                let _ = route_independent_security_effect(
                    &runtime.guard,
                    &mut runtime.app,
                    &mut runtime.security_coordinator,
                    effect,
                    runtime.editor_command.as_deref(),
                )
                .await;
            }
        } else if runtime.app.screen == Screen::Qa
            && qa_workspace_action(
                runtime.app.qa.view,
                runtime.app.qa.drilled,
                runtime.app.qa.searching,
                input,
            )
            .is_some()
        {
            let action = qa_workspace_action(
                runtime.app.qa.view,
                runtime.app.qa.drilled,
                runtime.app.qa.searching,
                input,
            )
            .expect("QA action was checked");
            if let Some(effect) = compatibility_workspace_action(&mut runtime.app, action) {
                let _ = route_independent_qa_effect(
                    &runtime.guard,
                    &mut runtime.app,
                    &mut runtime.qa_coordinator,
                    effect,
                    runtime.editor_command.as_deref(),
                )
                .await;
            }
        } else if runtime.app.screen == Screen::Maintenance
            && maintenance_workspace_action(
                runtime.app.maintenance.view,
                maintenance_row_count(&runtime.app),
                input,
            )
            .is_some()
        {
            let action = maintenance_workspace_action(
                runtime.app.maintenance.view,
                maintenance_row_count(&runtime.app),
                input,
            )
            .expect("Maintenance action was checked");
            if let Some(effect) = compatibility_workspace_action(&mut runtime.app, action) {
                let _ = route_independent_maintenance_effect(
                    &runtime.guard,
                    &mut runtime.app,
                    &mut runtime.maintenance_coordinator,
                    effect,
                    runtime.editor_command.as_deref(),
                )
                .await;
            }
        } else if runtime.app.screen == Screen::BuildEnvironment
            && runtime
                .app
                .build_environment_draft
                .as_ref()
                .is_some_and(|draft| draft.editing)
        {
            let action = match input {
                Input::Up => Some(Action::SelectBuildEnvironmentField { delta: -1 }),
                Input::Down => Some(Action::SelectBuildEnvironmentField { delta: 1 }),
                Input::Enter => Some(Action::ApplyBuildEnvironmentProfile),
                Input::Esc => Some(Action::CancelBuildEnvironmentEdit),
                Input::Backspace => Some(Action::BackspaceBuildEnvironmentField),
                Input::Char(c) => Some(Action::AppendBuildEnvironmentField(c)),
                _ => None,
            };
            if let Some(action) = action {
                let _ = compatibility_workspace_action(&mut runtime.app, action);
            }
        } else if runtime.app.screen == Screen::BuildEnvironment
            && build_environment_action(input).is_some()
        {
            let action =
                build_environment_action(input).expect("build environment action was checked");
            if matches!(action, Action::EnvironmentSetup(_)) {
                if let Some(effect) = compatibility_workspace_action(&mut runtime.app, action) {
                    runtime.environment_browser_io.submit(effect);
                }
                return Ok(Some(KeyRouteOutcome::ContinueLoop));
            }
            if let Some(Effect::VerifyBuildEnvironment {
                profile,
                generation,
            }) = compatibility_workspace_action(&mut runtime.app, action)
            {
                environment_operation::start(
                    &mut runtime.environment_operation,
                    profile,
                    generation,
                    runtime.backend_kind.clone(),
                    runtime.cancellation_timeout,
                );
            }
        } else if runtime.app.screen == Screen::Settings && settings_action(input).is_some() {
            if runtime.app.settings_selection == 0 && matches!(input, Input::Enter | Input::Right) {
                let _ = compatibility_workspace_action(&mut runtime.app, Action::OpenThemePicker);
                return Ok(Some(KeyRouteOutcome::ContinueLoop));
            }
            let action = settings_action(input).expect("settings action was checked");
            match compatibility_workspace_action(&mut runtime.app, action) {
                Some(Effect::PersistSettings) => {
                    let result = persist_settings(
                        runtime.session_path.as_deref(),
                        &mut runtime.session,
                        &runtime.app,
                        !runtime.color_forced_off,
                    );
                    let persistence_action = match result {
                        Ok(()) => Action::SettingsPersisted,
                        Err(error) => Action::SettingsPersistenceFailed(error.to_string()),
                    };
                    let _ = compatibility_workspace_action(&mut runtime.app, persistence_action);
                }
                Some(Effect::VerifyBuildEnvironment {
                    profile,
                    generation,
                }) => environment_operation::start(
                    &mut runtime.environment_operation,
                    profile,
                    generation,
                    runtime.backend_kind.clone(),
                    runtime.cancellation_timeout,
                ),
                _ => {}
            }
        } else if runtime.app.screen == Screen::Tasks
            && tasks_action(runtime.app.task_filter_editing, input).is_some()
        {
            let action = tasks_action(runtime.app.task_filter_editing, input)
                .expect("Tasks action was checked");
            let _ = compatibility_workspace_action(&mut runtime.app, action);
        } else if runtime.app.screen == Screen::Logs
            && log_workspace_action(&runtime.app, input).is_some()
        {
            let action = log_workspace_action(&runtime.app, input)
                .expect("Logs workspace action was checked");
            match compatibility_workspace_action(&mut runtime.app, action) {
                Some(Effect::OpenInEditor(path)) => {
                    open_in_editor(
                        &runtime.guard,
                        &mut runtime.app,
                        path,
                        runtime.editor_command.as_deref(),
                    )
                    .await;
                }
                Some(Effect::CopyToClipboard(content)) => {
                    copy_to_clipboard(&mut runtime.app, content).await;
                }
                _ => {}
            }
        } else if runtime.app.screen == Screen::Errors && errors_action(input).is_some() {
            let action = errors_action(input).expect("Errors action was checked");
            if let Some(Effect::OpenInEditor(path)) =
                compatibility_workspace_action(&mut runtime.app, action)
            {
                open_in_editor(
                    &runtime.guard,
                    &mut runtime.app,
                    path,
                    runtime.editor_command.as_deref(),
                )
                .await;
            }
        } else if route_dependency_workspace(runtime, input).await {
        } else {
            return Ok(None);
        }
        Ok(Some(KeyRouteOutcome::Handled))
    }
}
