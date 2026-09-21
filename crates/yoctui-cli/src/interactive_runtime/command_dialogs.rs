use super::*;

impl InteractiveRuntime {
    pub(super) async fn route_command_and_admin_dialogs(
        &mut self,
        input: Input,
    ) -> Result<Option<KeyRouteOutcome>> {
        let runtime = self;
        if runtime.app.command_palette_open {
            let global_search_edit = runtime.app.command_palette_mode
                == yoctui_model::CommandPaletteMode::GlobalRegexSearch
                && matches!(input, Input::Backspace | Input::CtrlU | Input::Char(_));
            let global_search_close = runtime.app.command_palette_mode
                == yoctui_model::CommandPaletteMode::GlobalRegexSearch
                && input == Input::Esc;
            let effect = match input {
                Input::Up => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::SelectCommandPalette { delta: -1 },
                ),
                Input::Down => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::SelectCommandPalette { delta: 1 },
                ),
                Input::Enter => {
                    compatibility_workspace_action(&mut runtime.app, Action::ActivateCommandPalette)
                }
                Input::Esc => {
                    compatibility_workspace_action(&mut runtime.app, Action::CloseCommandPalette)
                }
                Input::Backspace => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::BackspaceCommandPaletteQuery,
                ),
                Input::CtrlU => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::ClearCommandPaletteQuery,
                ),
                Input::Char(character) => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::AppendCommandPaletteQuery(character),
                ),
                _ => None,
            };
            if let Some(effect @ Effect::GetImageArtifacts(_)) = effect {
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
            } else if let Some(effect @ Effect::InspectSdkTools) = effect {
                begin_sdk_capability_operation(
                    &mut runtime.app,
                    runtime.sdk_tool_adapter.as_ref(),
                    &mut runtime.sdk_capability_operation,
                    effect,
                );
            } else if let Some(
                effect @ (Effect::InspectTestCapability | Effect::InspectResultToolCapability),
            ) = effect
            {
                let _ = runtime
                    .test_coordinator
                    .handle_effect(&mut runtime.app, effect)
                    .await;
            } else if let Some(effect @ Effect::Security(_)) = effect {
                let _ = route_independent_security_effect(
                    &runtime.guard,
                    &mut runtime.app,
                    &mut runtime.security_coordinator,
                    effect,
                    runtime.editor_command.as_deref(),
                )
                .await;
            } else if let Some(effect @ Effect::Qa(_)) = effect {
                let _ = route_independent_qa_effect(
                    &runtime.guard,
                    &mut runtime.app,
                    &mut runtime.qa_coordinator,
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
            } else if let Some(Effect::OpenInEditor(path)) = effect {
                open_in_editor(
                    &runtime.guard,
                    &mut runtime.app,
                    path,
                    runtime.editor_command.as_deref(),
                )
                .await;
                if let Some(action) = global_search_return_action(&runtime.app) {
                    let _ = compatibility_workspace_action(&mut runtime.app, action);
                }
            }
            if global_search_edit {
                begin_global_content_search(
                    &mut runtime.app,
                    &runtime.session_build_dir,
                    &mut runtime.global_content_search_operation,
                );
            } else if global_search_close
                && let Some(operation) = runtime.global_content_search_operation.take()
            {
                operation.cancellation.cancel();
            }
        } else if let Some(action) = global_search_action(&runtime.app, input) {
            let _ = compatibility_workspace_action(&mut runtime.app, action);
        } else if runtime.app.keymap_preferences_ui.open {
            let effect = keymap_preferences_action(&runtime.app, input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            match effect {
                Some(Effect::PersistSettings) => {
                    let result = persist_settings(
                        runtime.session_path.as_deref(),
                        &mut runtime.session,
                        &runtime.app,
                        !runtime.color_forced_off,
                    );
                    let action = match result {
                        Ok(()) => Action::SettingsPersisted,
                        Err(error) => Action::SettingsPersistenceFailed(error.to_string()),
                    };
                    let _ = compatibility_workspace_action(&mut runtime.app, action);
                }
                Some(Effect::CopyToClipboard(report)) => {
                    copy_to_clipboard(&mut runtime.app, report).await;
                }
                _ => {}
            }
        } else if let Some(Dialog::BuildEnvironmentCloneEditor(editor)) =
            runtime.app.active_dialog().cloned()
        {
            let action = match input {
                Input::Enter => Some(Action::ReviewBuildEnvironmentClone),
                Input::Esc if !editor.editing => Some(Action::CancelBuildEnvironmentClone),
                Input::Char('q') if !editor.editing => Some(Action::CancelBuildEnvironmentClone),
                input => popup_editor_action(editor.editing, input),
            };
            if let Some(Effect::CopyToClipboard(content)) =
                action.and_then(|action| compatibility_workspace_action(&mut runtime.app, action))
            {
                copy_to_clipboard(&mut runtime.app, content).await;
            }
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::BuildEnvironmentCloneReview(_))
        ) {
            let action = match input {
                Input::Enter => Some(Action::ConfirmBuildEnvironmentClone),
                Input::Esc => Some(Action::CancelBuildEnvironmentClone),
                _ => None,
            };
            if let Some(action) = action
                && let Some(Effect::CloneBuildEnvironment(plan)) =
                    compatibility_workspace_action(&mut runtime.app, action)
            {
                clone_operation::start(&mut runtime.app, &mut runtime.clone_operation, plan);
            }
        } else if let Some(Dialog::BuildEnvironmentEditor(editor)) =
            runtime.app.active_dialog().cloned()
        {
            let action = match input {
                Input::Enter => Some(Action::ApplyBuildEnvironmentEditor),
                Input::Char('q') | Input::Esc if !editor.editing => {
                    Some(Action::CloseBuildEnvironmentEditor)
                }
                input => popup_editor_action(editor.editing, input),
            };
            if let Some(Effect::CopyToClipboard(content)) =
                action.and_then(|action| compatibility_workspace_action(&mut runtime.app, action))
            {
                copy_to_clipboard(&mut runtime.app, content).await;
            }
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::ThemePicker { .. })
        ) {
            let action = match input {
                Input::Up | Input::Char('k') => Some(Action::SelectTheme { delta: -1 }),
                Input::Down | Input::Char('j') => Some(Action::SelectTheme { delta: 1 }),
                Input::Enter => Some(Action::ApplySelectedTheme),
                Input::Esc => Some(Action::CloseThemePicker),
                _ => None,
            };
            if let Some(action) = action
                && let Some(Effect::PersistSettings) =
                    compatibility_workspace_action(&mut runtime.app, action)
            {
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
        } else if let Some(Dialog::Maintenance(dialog)) = runtime.app.active_dialog().cloned() {
            let effect = maintenance_dialog_action(&dialog, input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(effect) = effect {
                if let Effect::CopyToClipboard(content) = effect {
                    copy_to_clipboard(&mut runtime.app, content).await;
                    return Ok(Some(KeyRouteOutcome::ContinueLoop));
                }
                let _ = route_independent_maintenance_effect(
                    &runtime.guard,
                    &mut runtime.app,
                    &mut runtime.maintenance_coordinator,
                    effect,
                    runtime.editor_command.as_deref(),
                )
                .await;
            }
        } else if let Some(Dialog::Security(dialog)) = runtime.app.active_dialog().cloned() {
            let effect = security_dialog_action(&dialog, input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(effect) = effect {
                if let Effect::CopyToClipboard(content) = effect {
                    copy_to_clipboard(&mut runtime.app, content).await;
                    return Ok(Some(KeyRouteOutcome::ContinueLoop));
                }
                let routed = route_independent_security_effect(
                    &runtime.guard,
                    &mut runtime.app,
                    &mut runtime.security_coordinator,
                    effect.clone(),
                    runtime.editor_command.as_deref(),
                )
                .await;
                if !routed {
                    match effect {
                        Effect::Security(SecurityEffect::StartBuild { id, request }) => {
                            if begin_security_build(
                                &mut runtime.backend,
                                &mut runtime.app,
                                &mut runtime.build_jobs,
                                id,
                                request,
                            )
                            .await
                            {
                                runtime.pending_security_build = Some(id);
                            }
                        }
                        Effect::Security(SecurityEffect::CancelSession(id))
                            if runtime.pending_security_build == Some(id) =>
                        {
                            if let Some(action) = runtime.build_jobs.request_cancellation() {
                                let _ = compatibility_workspace_action(&mut runtime.app, action);
                            }
                            if let Err(error) = runtime.backend.cancel_build().await {
                                let _ = compatibility_workspace_action(
                                    &mut runtime.app,
                                    Action::Security(SecurityAction::RejectCancellation {
                                        id,
                                        message: error.to_string(),
                                    }),
                                );
                                for action in runtime
                                    .build_jobs
                                    .cancellation_failed(error.to_string(), SystemTime::now())
                                {
                                    let _ =
                                        compatibility_workspace_action(&mut runtime.app, action);
                                }
                            }
                        }
                        Effect::Security(SecurityEffect::CancelSession(id)) => {
                            let _ = compatibility_workspace_action(
                                &mut runtime.app,
                                Action::Security(SecurityAction::RejectCancellation {
                                    id,
                                    message: "the CLI does not own this Security operation".into(),
                                }),
                            );
                        }
                        _ => {}
                    }
                }
            }
        } else if let Some(Dialog::Qa(dialog)) = runtime.app.active_dialog().cloned() {
            let effect = qa_dialog_action(&dialog, input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(effect) = effect {
                if let Effect::CopyToClipboard(content) = effect {
                    copy_to_clipboard(&mut runtime.app, content).await;
                    return Ok(Some(KeyRouteOutcome::ContinueLoop));
                }
                let routed = route_independent_qa_effect(
                    &runtime.guard,
                    &mut runtime.app,
                    &mut runtime.qa_coordinator,
                    effect.clone(),
                    runtime.editor_command.as_deref(),
                )
                .await;
                if !routed {
                    match effect {
                        Effect::Qa(QaEffect::StartBuild { session, request }) => {
                            if begin_qa_build(
                                &mut runtime.backend,
                                &mut runtime.app,
                                &mut runtime.build_jobs,
                                session,
                                request,
                            )
                            .await
                            {
                                runtime.pending_qa_build = Some(session);
                            }
                        }
                        Effect::Qa(QaEffect::CancelBuild { session, .. })
                            if runtime.pending_qa_build == Some(session) =>
                        {
                            if let Some(action) = runtime.build_jobs.request_cancellation() {
                                let _ = compatibility_workspace_action(&mut runtime.app, action);
                            }
                            if let Err(error) = runtime.backend.cancel_build().await {
                                let _ = compatibility_workspace_action(
                                    &mut runtime.app,
                                    Action::Qa(QaAction::RejectCancellation {
                                        session,
                                        message: error.to_string(),
                                    }),
                                );
                                for action in runtime
                                    .build_jobs
                                    .cancellation_failed(error.to_string(), SystemTime::now())
                                {
                                    let _ =
                                        compatibility_workspace_action(&mut runtime.app, action);
                                }
                            }
                        }
                        Effect::Qa(QaEffect::CancelBuild { session, .. }) => {
                            let _ = compatibility_workspace_action(
                                &mut runtime.app,
                                Action::Qa(QaAction::RejectCancellation {
                                    session,
                                    message: "the CLI does not own this QA managed build".into(),
                                }),
                            );
                        }
                        _ => {}
                    }
                }
            }
        } else {
            return Ok(None);
        }
        Ok(Some(KeyRouteOutcome::Handled))
    }
}
