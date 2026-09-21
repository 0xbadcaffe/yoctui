use super::*;

impl InteractiveRuntime {
    pub(super) async fn route_devtool_and_build_dialogs(
        &mut self,
        input: Input,
    ) -> Result<Option<KeyRouteOutcome>> {
        let runtime = self;
        if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::DevtoolResetConfirmation(_))
        ) {
            let effect = devtool_reset_confirmation_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(Effect::DevtoolReset(plan)) = effect {
                if submit_daemon_effect(
                    &mut runtime.daemon_runtime,
                    &mut runtime.app,
                    &Effect::DevtoolReset(plan.clone()),
                )
                .is_some()
                {
                    return Ok(Some(KeyRouteOutcome::ContinueLoop));
                }
                let operation = plan.operation();
                if begin_devtool_job(
                    &mut runtime.app,
                    &mut runtime.devtool_jobs,
                    &mut runtime.devtool_runner,
                    &runtime.session_build_dir,
                    runtime.cancellation_timeout,
                    None,
                    operation,
                )
                .await
                {
                    runtime.pending_devtool_reset = Some(plan.identity);
                }
            }
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::DevtoolUpdateConfirmation(_))
        ) {
            let effect = devtool_update_confirmation_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(Effect::DevtoolUpdateRecipe(identity)) = effect {
                if submit_daemon_effect(
                    &mut runtime.daemon_runtime,
                    &mut runtime.app,
                    &Effect::DevtoolUpdateRecipe(identity.clone()),
                )
                .is_some()
                {
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
                    DevtoolOperation::UpdateRecipe { recipe },
                )
                .await
                {
                    runtime.pending_devtool_update = Some(identity);
                }
            }
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::DevtoolFinishConfirmation(_))
        ) {
            let effect = devtool_finish_confirmation_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(Effect::DevtoolFinish(plan)) = effect {
                if submit_daemon_effect(
                    &mut runtime.daemon_runtime,
                    &mut runtime.app,
                    &Effect::DevtoolFinish(plan.clone()),
                )
                .is_some()
                {
                    return Ok(Some(KeyRouteOutcome::ContinueLoop));
                }
                let request = plan.request();
                if begin_devtool_job(
                    &mut runtime.app,
                    &mut runtime.devtool_jobs,
                    &mut runtime.devtool_runner,
                    &runtime.session_build_dir,
                    runtime.cancellation_timeout,
                    None,
                    request.into(),
                )
                .await
                {
                    runtime.pending_devtool_finish = Some(plan.identity);
                }
            }
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::DevtoolFinishPicker(_))
        ) {
            let _ = devtool_finish_picker_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::DevtoolDeployConfirmation(_))
        ) {
            let effect = devtool_deploy_confirmation_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(Effect::DevtoolDeploy(plan)) = effect {
                if submit_daemon_effect(
                    &mut runtime.daemon_runtime,
                    &mut runtime.app,
                    &Effect::DevtoolDeploy(plan.clone()),
                )
                .is_some()
                {
                    return Ok(Some(KeyRouteOutcome::ContinueLoop));
                }
                let request = plan.request();
                if begin_devtool_job(
                    &mut runtime.app,
                    &mut runtime.devtool_jobs,
                    &mut runtime.devtool_runner,
                    &runtime.session_build_dir,
                    runtime.cancellation_timeout,
                    None,
                    request.into(),
                )
                .await
                {
                    runtime.pending_devtool_deploy = Some(plan.identity);
                }
            }
        } else if matches!(runtime.app.active_dialog(), Some(Dialog::DevtoolDeploy(_))) {
            let _ = devtool_deploy_dialog_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::BbmaskConfirmation(_))
        ) {
            let effect = match input {
                Input::Enter => {
                    compatibility_workspace_action(&mut runtime.app, Action::ConfirmBbmaskWrite)
                }
                Input::Esc => {
                    compatibility_workspace_action(&mut runtime.app, Action::CancelBbmaskWrite)
                }
                _ => None,
            };
            if let Some(Effect::WriteBbmask(value)) = effect {
                match write_bbmask(&runtime.session_build_dir, value).await {
                    Ok(()) => {
                        refresh_workspace(
                            &mut runtime.backend,
                            &mut runtime.app,
                            "BBMASK saved and workspace metadata refreshed.",
                        )
                        .await
                    }
                    Err(error) => {
                        runtime.app.notification = Some(format!("Could not save BBMASK: {error}"))
                    }
                }
            }
        } else if let Some(Dialog::BbmaskEdit(editor)) = runtime.app.active_dialog().cloned() {
            let action = match input {
                Input::Enter => Some(Action::PreviewBbmaskEdit),
                Input::Char('q') | Input::Esc if !editor.editing => Some(Action::CancelBbmaskEdit),
                input => popup_editor_action(editor.editing, input),
            };
            if let Some(Effect::CopyToClipboard(content)) =
                action.and_then(|action| compatibility_workspace_action(&mut runtime.app, action))
            {
                copy_to_clipboard(&mut runtime.app, content).await;
            }
        } else if matches!(runtime.app.active_dialog(), Some(Dialog::BuildCompletion)) {
            let action = if input == Input::Enter
                && runtime.app.build.status == BuildStatus::Failed
                && runtime.app.build.errors > 0
            {
                Action::OpenBuildCompletionErrors
            } else {
                Action::DismissBuildCompletion
            };
            let _ = compatibility_workspace_action(&mut runtime.app, action);
        } else if matches!(runtime.app.active_dialog(), Some(Dialog::ImagePicker(_))) {
            let _ = match input {
                Input::Up => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::SelectImage { delta: -1 },
                ),
                Input::Down => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::SelectImage { delta: 1 },
                ),
                Input::Enter => {
                    compatibility_workspace_action(&mut runtime.app, Action::ConfirmImagePicker)
                }
                Input::Esc => {
                    compatibility_workspace_action(&mut runtime.app, Action::CancelImagePicker)
                }
                _ => None,
            };
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::SignatureTaskPicker(_))
        ) {
            let effect = signature_task_picker_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(effect @ Effect::GetSignatureDump(_)) = effect {
                begin_signature_operation(
                    &mut runtime.app,
                    &runtime.signature_adapter,
                    &mut runtime.signature_operation,
                    effect,
                );
            }
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::RecipeTaskPicker(_))
        ) {
            let _ = match input {
                Input::Up => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::SelectRecipeTask { delta: -1 },
                ),
                Input::Down => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::SelectRecipeTask { delta: 1 },
                ),
                Input::Enter => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::PreviewSelectedRecipeTask,
                ),
                Input::Esc => {
                    compatibility_workspace_action(&mut runtime.app, Action::CancelRecipeTaskPicker)
                }
                _ => None,
            };
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::RecipeTaskLogPicker(_))
        ) {
            let effect = match input {
                Input::Up => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::SelectRecipeTaskLog { delta: -1 },
                ),
                Input::Down => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::SelectRecipeTaskLog { delta: 1 },
                ),
                Input::Enter => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::OpenSelectedRecipeTaskLog,
                ),
                Input::Esc => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::CancelRecipeTaskLogPicker,
                ),
                _ => None,
            };
            if let Some(Effect::OpenInEditor(path)) = effect {
                open_in_editor(
                    &runtime.guard,
                    &mut runtime.app,
                    path,
                    runtime.editor_command.as_deref(),
                )
                .await;
            }
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::RecipePatchPicker(_))
        ) {
            let effect = match input {
                Input::Up => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::SelectRecipePatch { delta: -1 },
                ),
                Input::Down => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::SelectRecipePatch { delta: 1 },
                ),
                Input::Enter => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::OpenSelectedRecipePatch,
                ),
                Input::Esc => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::CancelRecipePatchPicker,
                ),
                _ => None,
            };
            if let Some(Effect::OpenInEditor(path)) = effect {
                open_in_editor(
                    &runtime.guard,
                    &mut runtime.app,
                    path,
                    runtime.editor_command.as_deref(),
                )
                .await;
            }
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::ConfigSourcePicker(_))
        ) {
            let effect = config_source_picker_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(Effect::OpenInEditor(path)) = effect {
                open_in_editor(
                    &runtime.guard,
                    &mut runtime.app,
                    path,
                    runtime.editor_command.as_deref(),
                )
                .await;
            }
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::ConfigScopePicker(_))
        ) {
            let effect = config_scope_picker_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(Effect::GetVariable(identity)) = effect {
                load_config_variable(&mut runtime.app, runtime.backend.as_mut(), identity).await;
            }
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::ConfigComparison(_))
        ) {
            if let Some(action) = config_compare_dialog_action(input) {
                let _ = compatibility_workspace_action(&mut runtime.app, action);
            }
        } else if let Some(Dialog::ConfigEdit { editor, .. }) = runtime.app.active_dialog().cloned()
        {
            let action = match input {
                Input::Enter => Some(Action::PreviewConfigEdit),
                Input::Char('q') | Input::Esc if !editor.editing => Some(Action::CancelConfigEdit),
                input => popup_editor_action(editor.editing, input),
            };
            if let Some(Effect::CopyToClipboard(content)) =
                action.and_then(|action| compatibility_workspace_action(&mut runtime.app, action))
            {
                copy_to_clipboard(&mut runtime.app, content).await;
            }
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::ConfigEditConfirmation(_))
        ) {
            if let Some(action) = config_edit_confirmation_action(input)
                && let Some(Effect::WriteConfigAssignment(request)) =
                    compatibility_workspace_action(&mut runtime.app, action)
            {
                execute_config_edit_write(
                    runtime.backend.as_mut(),
                    &mut runtime.app,
                    &runtime.session_build_dir,
                    request,
                )
                .await;
            }
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::RecipeTaskConfirmation(_))
        ) {
            let effect = match input {
                Input::Enter => {
                    compatibility_workspace_action(&mut runtime.app, Action::ConfirmRecipeTask)
                }
                Input::Esc => {
                    compatibility_workspace_action(&mut runtime.app, Action::CancelRecipeTask)
                }
                _ => None,
            };
            if let Some(Effect::Start(request)) = effect {
                begin_runtime_build(
                    &mut runtime.daemon_runtime,
                    &mut runtime.backend,
                    &mut runtime.app,
                    &mut runtime.build_jobs,
                    request,
                )
                .await;
            }
        } else if matches!(runtime.app.active_dialog(), Some(Dialog::BuildOptions)) {
            let effect = match input {
                Input::Char('b') => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::BeginBuildTargetTask(None),
                ),
                Input::Char('c') => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::BeginBuildTargetTask(Some("clean".into())),
                ),
                Input::Char('m') => compatibility_workspace_action(
                    &mut runtime.app,
                    Action::BeginBuildTargetTask(Some("menuconfig".into())),
                ),
                Input::Char('e') => {
                    compatibility_workspace_action(&mut runtime.app, Action::BeginBuildTargetEdit)
                }
                Input::Esc => {
                    compatibility_workspace_action(&mut runtime.app, Action::CloseBuildOptions)
                }
                _ => None,
            };
            if let Some(Effect::Start(request)) = effect {
                begin_runtime_build(
                    &mut runtime.daemon_runtime,
                    &mut runtime.backend,
                    &mut runtime.app,
                    &mut runtime.build_jobs,
                    request,
                )
                .await;
            }
        } else if let Some(Dialog::BuildTarget { editor, .. }) =
            runtime.app.active_dialog().cloned()
        {
            let action = match input {
                Input::Enter => Some(Action::ConfirmBuildTarget),
                Input::Char('q') | Input::Esc if !editor.editing => {
                    Some(Action::CancelBuildTargetEdit)
                }
                input => popup_editor_action(editor.editing, input),
            };
            let effect =
                action.and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            match effect {
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
        } else {
            return Ok(None);
        }
        Ok(Some(KeyRouteOutcome::Handled))
    }
}
