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
            let effect = qemu_launch_confirmation_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
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
        } else if matches!(runtime.app.active_dialog(), Some(Dialog::DtcCompile(_))) {
            let _ = dtc_compile_dialog_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
        } else if matches!(runtime.app.active_dialog(), Some(Dialog::TerminalLaunch(_))) {
            let effect = terminal_launch_dialog_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            match effect {
                Some(effect @ Effect::Terminal(_)) => {
                    if submit_daemon_effect(&mut runtime.daemon_runtime, &mut runtime.app, &effect)
                        .is_none()
                    {
                        if let Effect::Terminal(yoctui_model::TerminalEffect::Create {
                            kind: yoctui_model::TerminalCreationKind::GitUi,
                            program,
                            cwd,
                            arguments,
                            ..
                        }) = effect
                        {
                            if let Err(error) = runtime.guard.suspend() {
                                runtime.app.notification =
                                    Some(format!("Cannot open GitUI: {error}"));
                            } else {
                                let result = tokio::task::spawn_blocking(move || {
                                    std::process::Command::new(program)
                                        .args(arguments)
                                        .current_dir(cwd)
                                        .status()
                                })
                                .await;
                                let restored = runtime.guard.resume();
                                runtime.app.notification = Some(match (result, restored) {
                                    (_, Err(error)) => {
                                        format!("Cannot restore terminal: {error}")
                                    }
                                    (Ok(Ok(status)), Ok(())) if status.success() => {
                                        "GitUI closed; source status will refresh.".into()
                                    }
                                    (result, _) => {
                                        format!("GitUI finished: {result:?}")
                                    }
                                });
                            }
                        } else {
                            runtime.app.notification = Some("Embedded terminal unavailable: connect to the daemon or choose a detached terminal.".into());
                        }
                    }
                }
                Some(Effect::LaunchDetachedTerminal(request)) => {
                    match launch_detached_terminal(&request) {
                        Ok(()) => {
                            runtime.app.notification =
                                Some(format!("Detached terminal started for {}.", request.name));
                        }
                        Err(error) => {
                            runtime.app.notification =
                                Some(format!("Could not start detached terminal: {error}"));
                            let _ = update(
                                &mut runtime.app,
                                Action::DetachedTerminalAvailabilityDetected(
                                    detached_terminal_availability(),
                                ),
                            );
                        }
                    }
                }
                _ => {}
            }
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
                    if let Ok(content) = fs::read_to_string(path) {
                        let _ = compatibility_workspace_action(
                            &mut runtime.app,
                            Action::LoadRecipeEditorExternalContent(content),
                        );
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
                if submit_daemon_effect(
                    &mut runtime.daemon_runtime,
                    &mut runtime.app,
                    &Effect::DevtoolModify(identity.clone()),
                )
                .is_some()
                {
                    runtime.pending_daemon_devtool_modify = Some(identity);
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
            if let Some(effect @ (Effect::GetPackageInventory(_) | Effect::GetPackageDetail(_))) =
                effect
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
                runtime.inspect_kernel().await;
            } else if let Some(Effect::InspectFirmware) = effect {
                runtime.inspect_firmware().await;
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
            let _ = build_cancellation_confirmation_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
        } else if matches!(runtime.app.active_dialog(), Some(Dialog::QuitConfirmation)) {
            let _ = quit_confirmation_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
        } else if runtime.app.layer_browser.is_some()
            && !runtime.app.metadata_searching
            && runtime.app.focus != yoctui_model::FocusTarget::Dialog
        {
            let preview_focused = runtime
                .app
                .layer_browser
                .as_ref()
                .is_some_and(|browser| browser.preview_focused);
            let effect = match (preview_focused, input) {
                (true, Input::Up) => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::ScrollLayerBrowserPreview { delta: -1 },
                ),
                (true, Input::Down) => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::ScrollLayerBrowserPreview { delta: 1 },
                ),
                (true, Input::PageUp) => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::ScrollLayerBrowserPreview { delta: -10 },
                ),
                (true, Input::PageDown) => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::ScrollLayerBrowserPreview { delta: 10 },
                ),
                (true, Input::Left) => {
                    compatibility_workspace_action(&mut runtime.app, Action::FocusLayerBrowserTree)
                }
                (_, input) => match input {
                    Input::Tab => compatibility_workspace_action(
                        &mut runtime.app,
                        Action::CycleFocus { backwards: false },
                    ),
                    Input::BackTab => compatibility_workspace_action(
                        &mut runtime.app,
                        Action::CycleFocus { backwards: true },
                    ),
                    Input::Up => compatibility_workspace_action(
                        &mut runtime.app,
                        Action::SelectLayerBrowserEntry { delta: -1 },
                    ),
                    Input::Down => compatibility_workspace_action(
                        &mut runtime.app,
                        Action::SelectLayerBrowserEntry { delta: 1 },
                    ),
                    Input::PageUp => compatibility_workspace_action(
                        &mut runtime.app,
                        Action::SelectLayerBrowserEntry { delta: -10 },
                    ),
                    Input::PageDown => compatibility_workspace_action(
                        &mut runtime.app,
                        Action::SelectLayerBrowserEntry { delta: 10 },
                    ),
                    Input::Enter => {
                        compatibility_workspace_action(&mut runtime.app, Action::LayerBrowserEnter)
                    }
                    Input::Right | Input::Char('l') => {
                        compatibility_workspace_action(&mut runtime.app, Action::LayerBrowserExpand)
                    }
                    Input::Esc => {
                        compatibility_workspace_action(&mut runtime.app, Action::CloseLayerBrowser)
                    }
                    Input::Left | Input::Char('h') => {
                        compatibility_workspace_action(&mut runtime.app, Action::LayerBrowserUp)
                    }
                    Input::Char('r') => compatibility_workspace_action(
                        &mut runtime.app,
                        Action::RefreshLayerBrowser,
                    ),
                    Input::Char('e') => compatibility_workspace_action(
                        &mut runtime.app,
                        Action::EditSelectedLayerBrowserFile,
                    ),
                    Input::Char('.') => compatibility_workspace_action(
                        &mut runtime.app,
                        Action::ToggleLayerBrowserHidden,
                    ),
                    Input::Char('/') => compatibility_workspace_action(
                        &mut runtime.app,
                        Action::BeginMetadataSearch,
                    ),
                    Input::Char('i') => compatibility_workspace_action(
                        &mut runtime.app,
                        Action::SetLayerInspectorMode(LayerInspectorMode::Metadata),
                    ),
                    Input::Char('[') => compatibility_workspace_action(
                        &mut runtime.app,
                        Action::ScrollLayerBrowserPreview { delta: -10 },
                    ),
                    Input::Char(']') => compatibility_workspace_action(
                        &mut runtime.app,
                        Action::ScrollLayerBrowserPreview { delta: 10 },
                    ),
                    Input::Char('m') => compatibility_workspace_action(
                        &mut runtime.app,
                        Action::SetLayerInspectorMode(LayerInspectorMode::Metadata),
                    ),
                    Input::Char('d') => compatibility_workspace_action(
                        &mut runtime.app,
                        Action::SetLayerInspectorMode(LayerInspectorMode::Dependencies),
                    ),
                    _ => None,
                },
            };
            match effect {
                Some(Effect::LoadLayerBrowserDirectory {
                    layer,
                    root,
                    directory,
                }) => load_layer_browser_directory(&mut runtime.app, layer, root, directory).await,
                Some(Effect::LoadLayerBrowserPreview(path)) => {
                    load_layer_browser_preview(&mut runtime.app, path).await
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
