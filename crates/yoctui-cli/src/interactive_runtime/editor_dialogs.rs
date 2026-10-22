use super::*;

impl InteractiveRuntime {
    pub(super) async fn route_image_and_editor_dialogs(
        &mut self,
        input: Input,
    ) -> Result<Option<KeyRouteOutcome>> {
        let runtime = self;
        if let Some(Dialog::ImageConsole(dialog)) = runtime.app.active_dialog().cloned() {
            let effect = image_console_dialog_action(&dialog, input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(effect @ Effect::Terminal(_)) = effect {
                let _ =
                    submit_daemon_effect(&mut runtime.daemon_runtime, &mut runtime.app, &effect);
            }
        } else if matches!(runtime.app.active_dialog(), Some(Dialog::QemuLaunch(_))) {
            let editing = runtime
                .app
                .active_dialog()
                .is_some_and(|dialog| matches!(dialog, Dialog::QemuLaunch(state) if state.editing));
            let _ = qemu_launch_dialog_action(editing, input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::QemuLaunchConfirmation(_))
        ) {
            let size = runtime.terminal.size()?;
            if let Some(Dialog::QemuLaunchConfirmation(preview)) = runtime.app.active_dialog_mut() {
                yoctui_ui::clamp_qemu_launch_review_scroll(preview, size.width, size.height);
            }
            let effect = qemu_launch_confirmation_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(Dialog::QemuLaunchConfirmation(preview)) = runtime.app.active_dialog_mut() {
                yoctui_ui::clamp_qemu_launch_review_scroll(preview, size.width, size.height);
            }
            if let Some(effect @ Effect::Terminal(_)) = &effect {
                let _ = submit_daemon_effect(&mut runtime.daemon_runtime, &mut runtime.app, effect);
            }
            if let Some(effect @ Effect::StartQemuSession { .. }) = effect {
                if submit_daemon_effect(&mut runtime.daemon_runtime, &mut runtime.app, &effect)
                    .is_some()
                {
                    return Ok(Some(KeyRouteOutcome::ContinueLoop));
                }
                let Effect::StartQemuSession { id, request } = effect else {
                    unreachable!()
                };
                begin_qemu_job(
                    &mut runtime.app,
                    &mut runtime.qemu_operation,
                    &runtime.session_build_dir,
                    runtime.cancellation_timeout,
                    id,
                    request,
                )
                .await;
            }
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::QemuCancellationConfirmation(_))
        ) {
            let effect = qemu_cancellation_confirmation_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(effect @ Effect::CancelQemuSession(_)) = effect {
                if submit_daemon_effect(&mut runtime.daemon_runtime, &mut runtime.app, &effect)
                    .is_some()
                {
                    return Ok(Some(KeyRouteOutcome::ContinueLoop));
                }
                let Effect::CancelQemuSession(id) = effect else {
                    unreachable!()
                };
                begin_qemu_cancellation(&mut runtime.app, &mut runtime.qemu_operation, id);
            }
        } else if let Some(Dialog::DtcDecompile(dialog)) = runtime.app.active_dialog().cloned() {
            if let Some(effect) = dtc_decompile_dialog_action(&dialog, input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action))
            {
                runtime.environment_browser_io.submit(effect);
            }
        } else if matches!(runtime.app.active_dialog(), Some(Dialog::DtcCompile(_))) {
            let _ = dtc_compile_dialog_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
        } else if let Some(Dialog::YoctoUtility(dialog)) = runtime.app.active_dialog().cloned() {
            let _ = yocto_utility_dialog_action(&dialog, input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
        } else if matches!(runtime.app.active_dialog(), Some(Dialog::TerminalLaunch(_))) {
            return Ok(Some(runtime.route_terminal_launch_dialog(input).await?));
        } else if matches!(runtime.app.active_dialog(), Some(Dialog::RecipeEditor(_))) {
            let editor = runtime.app.active_dialog().and_then(|dialog| match dialog {
                Dialog::RecipeEditor(editor) => Some(editor.clone()),
                _ => None,
            });
            let effect = editor
                .as_ref()
                .and_then(|editor| recipe_editor_action(editor, input))
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            match effect {
                Some(Effect::LoadRecipeEditorFile(path)) => {
                    load_recipe_editor_file(&mut runtime.app, path).await;
                }
                Some(effect @ Effect::InspectRecipeEditorGitUi { .. }) => {
                    runtime.editor_gitui_io.submit(effect);
                }
                Some(Effect::SaveRecipeEditorFile {
                    root,
                    path,
                    content,
                    expected,
                }) => {
                    save_recipe_editor_file(&mut runtime.app, root, path, content, expected).await;
                }
                Some(Effect::OpenInEditor(path)) => {
                    open_in_editor(&runtime.guard, &mut runtime.app, path.clone(), Some("vim"))
                        .await;
                    if let Some(editor) = editor {
                        match crate::recipe_editor::read_source_editor_file(&editor, &path) {
                            Ok(content) => {
                                let _ = compatibility_workspace_action(
                                    &mut runtime.app,
                                    Action::LoadRecipeEditorExternalContent(content),
                                );
                            }
                            Err(error) => {
                                runtime.app.notification =
                                    Some(format!("Could not reload source file: {error}"));
                            }
                        }
                    }
                }
                Some(Effect::Start(request)) => {
                    begin_runtime_build(
                        &mut runtime.daemon_runtime,
                        &mut runtime.backend,
                        &mut runtime.app,
                        &mut runtime.build_jobs,
                        request,
                    )
                    .await;
                }
                Some(Effect::CopyToClipboard(content)) => {
                    copy_to_clipboard(&mut runtime.app, content).await;
                }
                _ => {}
            }
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::DevtoolModifyConfirmation(_))
        ) {
            let effect = devtool_modify_confirmation_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(Effect::DevtoolModify(identity)) = effect {
                let known_jobs = runtime.app.daemon.jobs.iter().map(|job| job.id).collect();
                if submit_daemon_effect(
                    &mut runtime.daemon_runtime,
                    &mut runtime.app,
                    &Effect::DevtoolModify(identity.clone()),
                )
                .is_some()
                {
                    runtime.pending_daemon_devtool_modify = Some((identity, known_jobs));
                    return Ok(Some(KeyRouteOutcome::ContinueLoop));
                }
                let recipe = identity.name.clone();
                if begin_devtool_job(
                    &mut runtime.app,
                    &mut runtime.devtool_jobs,
                    &mut runtime.devtool_runner,
                    &runtime.session_build_dir,
                    runtime.cancellation_timeout,
                    None,
                    DevtoolOperation::Modify { recipe },
                )
                .await
                {
                    runtime.pending_devtool_modify = Some(identity);
                }
            }
        } else if runtime.app.screen == Screen::Signatures
            && runtime.app.focus == yoctui_model::FocusTarget::Workspace
            && signature_workspace_action(input).is_some()
            && runtime.app.active_dialog().is_none()
            && runtime.app.notification.is_none()
        {
            let effect = signature_workspace_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            match effect {
                Some(effect @ (Effect::GetSignatureDump(_) | Effect::CompareSignatures(_))) => {
                    begin_signature_operation(
                        &mut runtime.app,
                        &runtime.signature_adapter,
                        &mut runtime.signature_operation,
                        effect,
                    )
                }
                Some(Effect::CancelSignatureOperation) => {
                    if let Some(operation) = runtime.signature_operation.as_ref() {
                        if operation.cancellation.cancel() {
                            runtime.app.notification =
                                Some("Signature cancellation requested.".into());
                        }
                    } else {
                        runtime.app.notification =
                            Some("No signature operation is running.".into());
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
                _ => {
                    if matches!(input, Input::Char('q') | Input::CtrlC) {
                        let _ = compatibility_workspace_action(&mut runtime.app, Action::Quit);
                    } else if input == Input::Char('?') {
                        let _ = compatibility_workspace_action(
                            &mut runtime.app,
                            Action::Open(Screen::Help),
                        );
                    }
                }
            }
        } else if !workspace_owns_focus_key(&runtime.app, input)
            && let Some(action) = pane_focus_route(&runtime.app, input)
        {
            let effect = compatibility_workspace_action(&mut runtime.app, action);
            if let Some(Effect::LoadLayerBrowserDirectory {
                layer,
                root,
                directory,
            }) = effect
            {
                runtime
                    .layer_browser_io
                    .directory(&mut runtime.app, layer, root, directory);
            } else if let Some(
                effect @ (Effect::GetPackageInventory(_) | Effect::GetPackageDetail(_)),
            ) = effect
            {
                begin_package_operation(
                    &mut runtime.app,
                    &runtime.package_adapter,
                    &mut runtime.package_operation,
                    effect,
                );
            } else if let Some(effect @ Effect::GetImageArtifacts(_)) = effect {
                begin_image_artifact_operation(
                    &mut runtime.app,
                    runtime.image_artifact_adapter.as_ref(),
                    &mut runtime.image_artifact_operation,
                    effect,
                );
            } else if let Some(effect @ Effect::GetRootfsComposition(_)) = effect {
                begin_rootfs_composition_operation(
                    runtime.backend.as_mut(),
                    &mut runtime.app,
                    &runtime.session_build_dir,
                    &mut runtime.rootfs_composition_operation,
                    effect,
                    runtime.daemon_attached,
                )
                .await;
            } else if let Some(effect @ Effect::InspectSdkTools) = effect {
                begin_sdk_capability_operation(
                    &mut runtime.app,
                    runtime.sdk_tool_adapter.as_ref(),
                    &mut runtime.sdk_capability_operation,
                    effect,
                );
            } else if let Some(Effect::InspectKernel) = effect {
                runtime.begin_platform_inspection(
                    platform_inspection_operation::PlatformInspectionRequest::Kernel,
                );
            } else if let Some(Effect::InspectFirmware) = effect {
                runtime.begin_platform_inspection(
                    platform_inspection_operation::PlatformInspectionRequest::Firmware,
                );
            } else if let Some(effect @ Effect::Security(_)) = effect {
                let _ = route_independent_security_effect(
                    &runtime.guard,
                    &mut runtime.app,
                    &mut runtime.security_coordinator,
                    effect,
                    runtime.editor_command.as_deref(),
                )
                .await;
            } else if let Some(effect @ Effect::Maintenance(_)) = effect {
                let _ = route_independent_maintenance_effect(
                    &runtime.guard,
                    &mut runtime.app,
                    &mut runtime.maintenance_coordinator,
                    effect,
                    runtime.editor_command.as_deref(),
                )
                .await;
            }
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::BuildCancellationConfirmation)
        ) {
            if let Some(Effect::Cancel) = build_cancellation_confirmation_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action))
            {
                runtime.cancel_build().await;
            }
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::ResolvedBuildRemovalConfirmation { .. })
        ) {
            let effect = resolved_build_removal_confirmation_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(Effect::RemoveSavedBuild(id)) = effect {
                let action = match build_archive::remove(&runtime.history_root, id.clone()).await {
                    Ok(_) => Action::ResolvedBuildRemoved { id },
                    Err(error) => Action::ResolvedBuildRemovalFailed {
                        message: error.to_string(),
                    },
                };
                let _ = update(&mut runtime.app, action);
            }
        } else if matches!(runtime.app.active_dialog(), Some(Dialog::QuitConfirmation)) {
            let _ = quit_confirmation_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
        } else if runtime.app.layer_browser.is_some()
            && (!runtime.app.metadata_searching || runtime.app.rootfs_browser_active())
            && (runtime.app.screen == Screen::Layers || runtime.app.rootfs_browser_active())
            && runtime.app.focus != yoctui_model::FocusTarget::Dialog
        {
            let preview_focused = runtime
                .app
                .layer_browser
                .as_ref()
                .is_some_and(|browser| browser.preview_focused);
            let action = browser_workspace_action(&runtime.app, preview_focused, input);
            let Some(action) = action else {
                // Unmapped keys belong to the global router (F-keys, menus,
                // quit, etc.), not to the file browser.
                return Ok(None);
            };
            let effect = compatibility_workspace_action(&mut runtime.app, action);
            match effect {
                Some(Effect::LoadLayerBrowserDirectory {
                    layer,
                    root,
                    directory,
                }) => runtime
                    .layer_browser_io
                    .directory(&mut runtime.app, layer, root, directory),
                Some(Effect::LoadLayerBrowserPreview(path)) => {
                    runtime.layer_browser_io.preview(&mut runtime.app, path)
                }
                Some(Effect::OpenLayerBrowserEditor { layer, root, file }) => {
                    if let Some(Effect::LoadRecipeEditorFile(path)) = compatibility_workspace_action(
                        &mut runtime.app,
                        Action::OpenRecipeEditor {
                            recipe: format!("Layer: {layer}"),
                            root,
                            files: vec![file],
                        },
                    ) {
                        load_recipe_editor_file(&mut runtime.app, path).await;
                    }
                }
                _ => {}
            }
        } else {
            return Ok(None);
        }
        Ok(Some(KeyRouteOutcome::Handled))
    }
}

fn browser_workspace_action(app: &App, preview_focused: bool, input: Input) -> Option<Action> {
    if app.screen == Screen::Images {
        return rootfs_browser_action(app, input);
    }
    if preview_focused {
        match input {
            Input::Up => return Some(Action::ScrollLayerBrowserPreview { delta: -1 }),
            Input::Down => return Some(Action::ScrollLayerBrowserPreview { delta: 1 }),
            Input::PageUp => return Some(Action::ScrollLayerBrowserPreview { delta: -10 }),
            Input::PageDown => return Some(Action::ScrollLayerBrowserPreview { delta: 10 }),
            Input::Home => return Some(Action::ScrollLayerBrowserPreview { delta: isize::MIN }),
            Input::End => return Some(Action::ScrollLayerBrowserPreview { delta: isize::MAX }),
            Input::Left => return Some(Action::FocusLayerBrowserTree),
            _ => {}
        }
    }
    match input {
        Input::Tab => Some(Action::CycleFocus { backwards: false }),
        Input::BackTab => Some(Action::CycleFocus { backwards: true }),
        _ => yoctui_app::layer_tree_action(app.metadata_searching, input),
    }
}

#[cfg(test)]
#[path = "../tests/interactive_runtime/browser_keys.rs"]
mod browser_key_tests;
