use super::*;

impl InteractiveRuntime {
    pub(super) async fn route_sdk_test_wic_dialogs(
        &mut self,
        input: Input,
    ) -> Result<Option<KeyRouteOutcome>> {
        let runtime = self;
        if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::SdkBuildConfirmation(_))
        ) {
            let effect = sdk_build_confirmation_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(Effect::Start(request)) = effect {
                let tracked = sdk_build_is_populate(&request);
                if begin_runtime_build(
                    &mut runtime.daemon_runtime,
                    &mut runtime.backend,
                    &mut runtime.app,
                    &mut runtime.build_jobs,
                    request.clone(),
                )
                .await
                    && tracked
                {
                    runtime.pending_sdk_build = Some(request);
                }
            }
        } else if let Some(Dialog::SdkPublishTomlEditor(editor)) =
            runtime.app.active_dialog().cloned()
        {
            let action = match input {
                Input::Enter => Some(Action::PreviewSdkPublish),
                Input::Char('q') | Input::Esc if !editor.editing => Some(Action::CancelSdkPublish),
                input => popup_editor_action(editor.editing, input),
            };
            if let Some(Effect::CopyToClipboard(content)) =
                action.and_then(|action| compatibility_workspace_action(&mut runtime.app, action))
            {
                copy_to_clipboard(&mut runtime.app, content).await;
            }
        } else if matches!(runtime.app.active_dialog(), Some(Dialog::SdkPublish(_))) {
            let _ = sdk_publish_dialog_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::SdkPublishConfirmation(_))
        ) {
            let effect = sdk_publish_confirmation_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(effect @ Effect::StartSdkSession { .. }) = effect {
                if submit_daemon_effect(&mut runtime.daemon_runtime, &mut runtime.app, &effect)
                    .is_some()
                {
                    return Ok(Some(KeyRouteOutcome::ContinueLoop));
                }
                let Effect::StartSdkSession { id, operation } = effect else {
                    unreachable!()
                };
                begin_sdk_job(
                    &mut runtime.app,
                    &mut runtime.sdk_operation,
                    runtime.sdk_tool_adapter.as_ref(),
                    runtime.cancellation_timeout,
                    SDK_TOOL_OPERATION_TIMEOUT,
                    id,
                    operation,
                );
            }
        } else if let Some(Dialog::SdkNativeTomlEditor(editor)) =
            runtime.app.active_dialog().cloned()
        {
            let action = match input {
                Input::Enter => Some(Action::PreviewSdkNative),
                Input::Char('q') | Input::Esc if !editor.editing => Some(Action::CancelSdkNative),
                input => popup_editor_action(editor.editing, input),
            };
            if let Some(Effect::CopyToClipboard(content)) =
                action.and_then(|action| compatibility_workspace_action(&mut runtime.app, action))
            {
                copy_to_clipboard(&mut runtime.app, content).await;
            }
        } else if let Some(Dialog::SdkNative(dialog)) = runtime.app.active_dialog() {
            let editing = dialog.editing;
            let _ = sdk_native_dialog_action(editing, input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::SdkNativeConfirmation(_))
        ) {
            let effect = sdk_native_confirmation_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(effect @ Effect::StartSdkSession { .. }) = effect {
                if submit_daemon_effect(&mut runtime.daemon_runtime, &mut runtime.app, &effect)
                    .is_some()
                {
                    return Ok(Some(KeyRouteOutcome::ContinueLoop));
                }
                let Effect::StartSdkSession { id, operation } = effect else {
                    unreachable!()
                };
                begin_sdk_job(
                    &mut runtime.app,
                    &mut runtime.sdk_operation,
                    runtime.sdk_tool_adapter.as_ref(),
                    runtime.cancellation_timeout,
                    SDK_TOOL_OPERATION_TIMEOUT,
                    id,
                    operation,
                );
            }
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::SdkCancellationConfirmation(_))
        ) {
            let effect = sdk_cancellation_confirmation_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(effect @ Effect::CancelSdkSession(_)) = effect {
                if submit_daemon_effect(&mut runtime.daemon_runtime, &mut runtime.app, &effect)
                    .is_some()
                {
                    return Ok(Some(KeyRouteOutcome::ContinueLoop));
                }
                let Effect::CancelSdkSession(id) = effect else {
                    unreachable!()
                };
                begin_sdk_cancellation(&mut runtime.app, &mut runtime.sdk_operation, id);
            }
        } else if let Some(Dialog::TestLaunchTomlEditor { editor, .. }) =
            runtime.app.active_dialog().cloned()
        {
            let action = match input {
                Input::Enter => Some(Action::PreviewTestLaunch),
                Input::Char('q') | Input::Esc if !editor.editing => Some(Action::CancelTestLaunch),
                input => popup_editor_action(editor.editing, input),
            };
            if let Some(Effect::CopyToClipboard(content)) =
                action.and_then(|action| compatibility_workspace_action(&mut runtime.app, action))
            {
                copy_to_clipboard(&mut runtime.app, content).await;
            }
        } else if let Some(Dialog::TestLaunch(dialog)) = runtime.app.active_dialog() {
            let editing = dialog.editing;
            let _ = test_launch_dialog_action(editing, input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::TestLaunchConfirmation(_))
        ) {
            let effect = test_launch_confirmation_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            match effect {
                Some(effect @ Effect::StartTestSession { .. }) => {
                    if submit_daemon_effect(&mut runtime.daemon_runtime, &mut runtime.app, &effect)
                        .is_none()
                    {
                        let _ = runtime
                            .test_coordinator
                            .handle_effect(&mut runtime.app, effect)
                            .await;
                    }
                }
                Some(Effect::StartTestBuildSession {
                    id,
                    family: _,
                    request,
                }) => {
                    if begin_test_build(
                        &mut runtime.backend,
                        &mut runtime.app,
                        &mut runtime.build_jobs,
                        id,
                        request,
                    )
                    .await
                    {
                        runtime.pending_test_build = Some(id);
                    }
                }
                _ => {}
            }
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::TestCancellationConfirmation(_))
        ) {
            let effect = test_cancellation_confirmation_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(effect @ Effect::CancelTestSession(id)) = effect
                && submit_daemon_effect(&mut runtime.daemon_runtime, &mut runtime.app, &effect)
                    .is_none()
                && !runtime
                    .test_coordinator
                    .handle_effect(&mut runtime.app, effect)
                    .await
                && runtime.pending_test_build == Some(id)
            {
                if let Some(action) = runtime.build_jobs.request_cancellation() {
                    let _ = compatibility_workspace_action(&mut runtime.app, action);
                }
                if let Err(error) = runtime.backend.cancel_build().await {
                    let _ = compatibility_workspace_action(
                        &mut runtime.app,
                        Action::RejectTestSessionCancellation {
                            id,
                            message: error.to_string(),
                        },
                    );
                    for action in runtime
                        .build_jobs
                        .cancellation_failed(error.to_string(), SystemTime::now())
                    {
                        let _ = compatibility_workspace_action(&mut runtime.app, action);
                    }
                }
            }
        } else if let Some(Dialog::TestResultImportTomlEditor { editor, .. }) =
            runtime.app.active_dialog().cloned()
        {
            let action = match input {
                Input::Enter => Some(Action::ConfirmTestResultImport),
                Input::Char('q') | Input::Esc if !editor.editing => {
                    Some(Action::CancelTestResultImport)
                }
                input => popup_editor_action(editor.editing, input),
            };
            if let Some(effect) =
                action.and_then(|action| compatibility_workspace_action(&mut runtime.app, action))
            {
                match effect {
                    Effect::CopyToClipboard(content) => {
                        copy_to_clipboard(&mut runtime.app, content).await;
                    }
                    effect => {
                        let _ = runtime
                            .test_coordinator
                            .handle_effect(&mut runtime.app, effect)
                            .await;
                    }
                }
            }
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::TestResultImport(_))
        ) {
            let effect = test_result_import_dialog_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(effect) = effect {
                let _ = runtime
                    .test_coordinator
                    .handle_effect(&mut runtime.app, effect)
                    .await;
            }
        } else if let Some(Dialog::TestComparisonTomlEditor { editor, .. }) =
            runtime.app.active_dialog().cloned()
        {
            let action = match input {
                Input::Enter => Some(Action::PreviewTestComparison),
                Input::Char('q') | Input::Esc if !editor.editing => {
                    Some(Action::CancelTestComparison)
                }
                input => popup_editor_action(editor.editing, input),
            };
            if let Some(Effect::CopyToClipboard(content)) =
                action.and_then(|action| compatibility_workspace_action(&mut runtime.app, action))
            {
                copy_to_clipboard(&mut runtime.app, content).await;
            }
        } else if matches!(runtime.app.active_dialog(), Some(Dialog::TestComparison(_))) {
            let _ = test_comparison_dialog_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::TestComparisonConfirmation(_))
        ) {
            let effect = test_comparison_confirmation_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(effect) = effect {
                let _ = runtime
                    .test_coordinator
                    .handle_effect(&mut runtime.app, effect)
                    .await;
            }
        } else if let Some(Dialog::TestJunitTomlEditor { editor, .. }) =
            runtime.app.active_dialog().cloned()
        {
            let action = if editor.editing {
                match input {
                    Input::Esc => Some(Action::ToggleTestJunitTomlEditor),
                    Input::Enter => Some(Action::PreviewTestJunitExport),
                    Input::Backspace => Some(Action::BackspaceTestJunitTomlEditor),
                    Input::Left => Some(Action::MoveTestJunitTomlEditorLeft),
                    Input::Right => Some(Action::MoveTestJunitTomlEditorRight),
                    Input::Up => Some(Action::MoveTestJunitTomlEditorUp),
                    Input::Down => Some(Action::MoveTestJunitTomlEditorDown),
                    Input::Home => Some(Action::MoveTestJunitTomlEditorHome),
                    Input::End => Some(Action::MoveTestJunitTomlEditorEnd),
                    Input::CtrlC => Some(Action::CopyTestJunitTomlEditor),
                    Input::CtrlV => Some(Action::PasteTestJunitTomlEditor),
                    Input::Char(character) => Some(Action::AppendTestJunitTomlEditor(character)),
                    _ => None,
                }
            } else {
                match input {
                    Input::Char('i') => Some(Action::ToggleTestJunitTomlEditor),
                    Input::Char('e') => Some(Action::SelectTestJunitDestination),
                    Input::Left | Input::Char('h') => Some(Action::MoveTestJunitTomlEditorLeft),
                    Input::Right | Input::Char('l') => Some(Action::MoveTestJunitTomlEditorRight),
                    Input::Up | Input::Char('k') => Some(Action::MoveTestJunitTomlEditorUp),
                    Input::Down | Input::Char('j') => Some(Action::MoveTestJunitTomlEditorDown),
                    Input::Home => Some(Action::MoveTestJunitTomlEditorHome),
                    Input::End => Some(Action::MoveTestJunitTomlEditorEnd),
                    Input::CtrlC => Some(Action::CopyTestJunitTomlEditor),
                    Input::Char('q') | Input::Esc => Some(Action::CancelTestJunitExport),
                    Input::Enter => Some(Action::PreviewTestJunitExport),
                    _ => None,
                }
            };
            if let Some(effect) =
                action.and_then(|action| compatibility_workspace_action(&mut runtime.app, action))
            {
                match effect {
                    Effect::CopyToClipboard(content) => {
                        copy_to_clipboard(&mut runtime.app, content).await;
                    }
                    effect => {
                        let _ = runtime
                            .test_coordinator
                            .handle_effect(&mut runtime.app, effect)
                            .await;
                    }
                }
            }
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::TestJunitExport(_))
        ) {
            let effect = test_junit_dialog_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(effect) = effect {
                let _ = runtime
                    .test_coordinator
                    .handle_effect(&mut runtime.app, effect)
                    .await;
            }
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::TestJunitExportConfirmation(_))
        ) {
            let effect = test_junit_confirmation_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(effect) = effect {
                let _ = runtime
                    .test_coordinator
                    .handle_effect(&mut runtime.app, effect)
                    .await;
            }
        } else if let Some(Dialog::WicCreateTomlEditor { editor, .. }) =
            runtime.app.active_dialog().cloned()
        {
            let action = match input {
                Input::Enter => Some(Action::PreviewWicCreate),
                Input::Char('q') | Input::Esc if !editor.editing => Some(Action::CancelWicCreate),
                input => popup_editor_action(editor.editing, input),
            };
            if let Some(Effect::CopyToClipboard(content)) =
                action.and_then(|action| compatibility_workspace_action(&mut runtime.app, action))
            {
                copy_to_clipboard(&mut runtime.app, content).await;
            }
        } else if matches!(runtime.app.active_dialog(), Some(Dialog::WicCreate(_))) {
            let editing = runtime
                .app
                .active_dialog()
                .is_some_and(|dialog| matches!(dialog, Dialog::WicCreate(state) if state.editing));
            let _ = wic_create_dialog_action(editing, input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::WicCreateConfirmation(_))
        ) {
            let effect = wic_create_confirmation_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(effect @ Effect::StartWicSession { .. }) = effect {
                if submit_daemon_effect(&mut runtime.daemon_runtime, &mut runtime.app, &effect)
                    .is_some()
                {
                    return Ok(Some(KeyRouteOutcome::ContinueLoop));
                }
                let Effect::StartWicSession { id, operation } = effect else {
                    unreachable!()
                };
                begin_wic_job(
                    &mut runtime.app,
                    &mut runtime.wic_operation,
                    &runtime.wic_device_inspector,
                    &runtime.session_build_dir,
                    runtime.cancellation_timeout,
                    id,
                    operation,
                )
                .await;
            }
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::WicDevicePicker(_))
        ) {
            let _ = wic_device_picker_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
        } else if matches!(runtime.app.active_dialog(), Some(Dialog::WicWritePhrase(_))) {
            let _ = wic_write_phrase_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
        } else if matches!(
            runtime.app.active_dialog(),
            Some(Dialog::WicWriteConfirmation(_))
        ) {
            let effect = wic_write_confirmation_action(input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(Effect::StartWicSession { id, operation }) = effect {
                begin_wic_job(
                    &mut runtime.app,
                    &mut runtime.wic_operation,
                    &runtime.wic_device_inspector,
                    &runtime.session_build_dir,
                    runtime.cancellation_timeout,
                    id,
                    operation,
                )
                .await;
            }
        } else if let Some(Dialog::WicCancellationConfirmation {
            id,
            incomplete_device_warning,
        }) = runtime.app.active_dialog().cloned()
        {
            let effect = wic_cancellation_confirmation_action(id, incomplete_device_warning, input)
                .and_then(|action| compatibility_workspace_action(&mut runtime.app, action));
            if let Some(effect @ Effect::CancelWicSession(_)) = effect {
                if submit_daemon_effect(&mut runtime.daemon_runtime, &mut runtime.app, &effect)
                    .is_some()
                {
                    return Ok(Some(KeyRouteOutcome::ContinueLoop));
                }
                let Effect::CancelWicSession(id) = effect else {
                    unreachable!()
                };
                begin_wic_cancellation(&mut runtime.app, &mut runtime.wic_operation, id);
            }
        } else {
            return Ok(None);
        }
        Ok(Some(KeyRouteOutcome::Handled))
    }
}
