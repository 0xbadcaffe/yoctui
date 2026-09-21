use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::CancelSdkPublish => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::SdkPublish(_) | Dialog::SdkPublishTomlEditor(_))
            ) {
                close_dialog(app);
            }
        }
        Action::CancelSdkPublishPreview => {
            if matches!(app.active_dialog(), Some(Dialog::SdkPublishConfirmation(_))) {
                close_dialog(app);
            }
        }
        Action::ConfirmSdkPublish => {
            let Some(Dialog::SdkPublishConfirmation(preview)) = app.active_dialog().cloned() else {
                return None;
            };
            let valid = app.selected_sdk_artifact().is_some_and(|artifact| {
                artifact.kind == SdkArtifactKind::Installer
                    && artifact.identity == preview.request.artifact
            }) && app.sdk_tool_capability.publish_executable().as_ref()
                == Ok(&preview.request.executable);
            if !valid {
                app.notification = Some("The SDK publication preview is stale.".into());
                return None;
            }
            close_dialog(app);
            return queue_sdk_session(app, SdkOperation::Publish(preview.request));
        }
        Action::BeginSdkNative => {
            let mut editor = PopupEditor::new("mode = \"find-sysroot\"\nworkspace = \"\"\nrecipe = \"\"\ntool = \"\"\narguments = \"\"\n".into());
            let _ = editor.select_toml_value("mode");
            open_dialog(app, Dialog::SdkNativeTomlEditor(editor));
        }
        Action::ToggleSdkNativeTomlEditor => {
            if let Some(Dialog::SdkNativeTomlEditor(editor)) = app.active_dialog_mut() {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendSdkNativeTomlEditor(character) => {
            if let Some(Dialog::SdkNativeTomlEditor(editor)) = app.active_dialog_mut()
                && editor.editing
                && !character.is_control()
                && editor.text.len() < 8_192
            {
                editor.insert(&character.to_string());
            }
        }
        Action::BackspaceSdkNativeTomlEditor => {
            if let Some(Dialog::SdkNativeTomlEditor(editor)) = app.active_dialog_mut()
                && editor.editing
            {
                editor.backspace();
            }
        }
        Action::UpdateSdkNativeDraft(draft) => {
            if matches!(app.active_dialog(), Some(Dialog::SdkNative(_))) {
                replace_dialog(app, Dialog::SdkNative(SdkNativeDialog::new(draft)));
            }
        }
        Action::SelectSdkNativeField { delta } => {
            if let Some(Dialog::SdkNative(dialog)) = app.active_dialog_mut()
                && !dialog.editing
            {
                dialog.selected_field = dialog.selected_field.shifted(delta);
                dialog.validation_error = None;
            }
        }
        Action::ActivateSdkNativeField => {
            if let Some(Dialog::SdkNative(dialog)) = app.active_dialog_mut() {
                if dialog.selected_field == SdkNativeField::Mode {
                    dialog.cycle_mode();
                } else if dialog.selected_field == SdkNativeField::Tool
                    && dialog.draft.mode == SdkNativeMode::FindSysroot
                {
                    dialog.validation_error =
                        Some("Tool is not applicable in find-native-sysroot mode.".into());
                } else {
                    dialog.editing = true;
                    dialog.validation_error = None;
                }
            }
        }
        Action::CycleSdkNativeMode => {
            if let Some(Dialog::SdkNative(dialog)) = app.active_dialog_mut()
                && !dialog.editing
                && dialog.selected_field == SdkNativeField::Mode
            {
                dialog.cycle_mode();
            }
        }
        Action::AppendSdkNativeField(character) => {
            if let Some(Dialog::SdkNative(dialog)) = app.active_dialog_mut()
                && dialog.editing
                && !character.is_control()
            {
                let arguments = dialog.selected_field == SdkNativeField::Arguments;
                if let Some((text, bound)) = dialog.selected_text_mut()
                    && text.len() + character.len_utf8() <= bound
                {
                    text.push(character);
                    if arguments {
                        dialog.synchronize_arguments();
                    }
                    dialog.validation_error = None;
                }
            }
        }
        Action::BackspaceSdkNativeField => {
            if let Some(Dialog::SdkNative(dialog)) = app.active_dialog_mut()
                && dialog.editing
            {
                let arguments = dialog.selected_field == SdkNativeField::Arguments;
                if let Some((text, _)) = dialog.selected_text_mut() {
                    text.pop();
                    if arguments {
                        dialog.synchronize_arguments();
                    }
                    dialog.validation_error = None;
                }
            }
        }
        Action::FinishSdkNativeFieldEdit => {
            if let Some(Dialog::SdkNative(dialog)) = app.active_dialog_mut() {
                dialog.synchronize_arguments();
                dialog.editing = false;
            }
        }
        Action::PreviewSdkNative => {
            if let Some(Dialog::SdkNativeTomlEditor(editor)) = app.active_dialog().cloned() {
                let request = (|| {
                    let fields = popup_toml_fields(&editor.text)?;
                    let mode = match fields.get("mode").map(String::as_str) {
                        Some("find-sysroot") => SdkNativeMode::FindSysroot,
                        Some("run-native") => SdkNativeMode::RunNative,
                        _ => return Err("`mode` must be find-sysroot or run-native.".to_owned()),
                    };
                    let value = |key: &str| {
                        fields
                            .get(key)
                            .cloned()
                            .ok_or_else(|| format!("Missing `{key}`."))
                    };
                    let workspace = value("workspace")?;
                    let recipe = value("recipe")?;
                    let tool = value("tool")?;
                    let arguments = value("arguments")?
                        .split_ascii_whitespace()
                        .map(str::to_owned)
                        .collect();
                    let executable = app
                        .sdk_tool_capability
                        .executable_for(mode)
                        .map_err(str::to_owned)?;
                    Ok(SdkNativeRequest {
                        executable,
                        mode,
                        extracted_root: (!workspace.is_empty()).then(|| PathBuf::from(workspace)),
                        recipe,
                        tool: (mode == SdkNativeMode::RunNative).then_some(tool),
                        arguments,
                    })
                })();
                match request
                    .and_then(|request| SdkNativePreview::new(request).map_err(str::to_owned))
                {
                    Ok(preview) => replace_dialog(app, Dialog::SdkNativeConfirmation(preview)),
                    Err(message) => app.notification = Some(message),
                }
                return None;
            }
            let Some(Dialog::SdkNative(dialog)) = app.active_dialog().cloned() else {
                return None;
            };
            let draft = dialog.draft;
            let executable = app.sdk_tool_capability.executable_for(draft.mode);
            let request = executable.map(|executable| SdkNativeRequest {
                executable,
                mode: draft.mode,
                extracted_root: (!draft.extracted_root.is_empty())
                    .then(|| PathBuf::from(draft.extracted_root)),
                recipe: draft.recipe,
                tool: (draft.mode == SdkNativeMode::RunNative).then_some(draft.tool),
                arguments: draft.arguments,
            });
            match request.and_then(SdkNativePreview::new) {
                Ok(preview) => replace_dialog(app, Dialog::SdkNativeConfirmation(preview)),
                Err(message) => app.notification = Some(message.into()),
            }
        }
        Action::CancelSdkNative => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::SdkNative(_) | Dialog::SdkNativeTomlEditor(_))
            ) {
                close_dialog(app);
            }
        }
        Action::CancelSdkNativePreview => {
            if matches!(app.active_dialog(), Some(Dialog::SdkNativeConfirmation(_))) {
                close_dialog(app);
            }
        }
        Action::ConfirmSdkNative => {
            let Some(Dialog::SdkNativeConfirmation(preview)) = app.active_dialog().cloned() else {
                return None;
            };
            if SdkNativePreview::new(preview.request.clone()).as_ref() != Ok(&preview)
                || app
                    .sdk_tool_capability
                    .executable_for(preview.request.mode)
                    .as_ref()
                    != Ok(&preview.request.executable)
            {
                app.notification = Some("The SDK native-tool preview is stale.".into());
                return None;
            }
            close_dialog(app);
            return queue_sdk_session(app, SdkOperation::Native(preview.request));
        }
        Action::SdkSessionStarting { id, started_at } => {
            let Some(job_id) = sdk_job_id(app, id) else {
                note_stale_sdk_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Queued], |job| {
                    job.status = BackgroundJobStatus::Starting;
                    job.started_at = Some(started_at);
                });
        }
        Action::SdkSessionRunning { id } => {
            let Some(job_id) = sdk_job_id(app, id) else {
                note_stale_sdk_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Starting], |job| {
                    job.status = BackgroundJobStatus::Running;
                });
        }
        Action::AppendSdkSessionOutput {
            id,
            stream,
            line,
            truncated,
            timestamp,
        } => {
            let Some(job_id) = sdk_job_id(app, id) else {
                note_stale_sdk_event(app);
                return None;
            };
            app.background_jobs.append_output(
                job_id,
                BackgroundJobOutputEntry {
                    severity: if stream == SdkOutputStream::Stderr {
                        Severity::Warning
                    } else {
                        Severity::Info
                    },
                    message: line,
                    source: if stream == SdkOutputStream::Stderr {
                        BackgroundJobOutputSource::Stderr
                    } else {
                        BackgroundJobOutputSource::Stdout
                    },
                    truncated,
                    timestamp,
                },
            );
        }
        Action::CompleteSdkSession {
            id,
            exit_code,
            artifacts,
            finished_at,
        } => {
            let Some(job_id) =
                mutate_sdk_session(app, id, |session| session.exit_code = Some(exit_code))
            else {
                note_stale_sdk_event(app);
                return None;
            };
            app.background_jobs.update_if(
                job_id,
                &[
                    BackgroundJobStatus::Starting,
                    BackgroundJobStatus::Running,
                    BackgroundJobStatus::Cancelling,
                ],
                |job| {
                    job.status = BackgroundJobStatus::Succeeded;
                    job.finished_at = Some(finished_at);
                    job.result = Some(BackgroundJobResult {
                        summary: "SDK operation completed".into(),
                        artifacts,
                    });
                },
            );
        }
        Action::FailSdkSession {
            id,
            message,
            exit_code,
            finished_at,
        } => {
            let Some(job_id) = mutate_sdk_session(app, id, |session| {
                session.exit_code = exit_code;
                session.error_detail = Some(message.clone());
            }) else {
                note_stale_sdk_event(app);
                return None;
            };
            app.background_jobs.update_if(
                job_id,
                &[
                    BackgroundJobStatus::Queued,
                    BackgroundJobStatus::Starting,
                    BackgroundJobStatus::Running,
                    BackgroundJobStatus::Cancelling,
                ],
                |job| {
                    job.status = BackgroundJobStatus::Failed;
                    job.finished_at = Some(finished_at);
                    job.error = Some(BackgroundJobError {
                        summary: "SDK operation failed".into(),
                        detail: Some(message),
                    });
                },
            );
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
