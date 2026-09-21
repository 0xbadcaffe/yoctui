use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::ConfirmWicDeviceSelection => {
            let Some(Dialog::WicDevicePicker(dialog)) = app.active_dialog().cloned() else {
                return None;
            };
            let Some(device) = app.selected_wic_device() else {
                app.notification = Some("No eligible Wic device is available to select.".into());
                return None;
            };
            let request_matches = matches!(
                &app.wic_devices,
                WicDeviceInventoryState::Available { request, .. }
                    | WicDeviceInventoryState::Partial { request, .. }
                    if request == &dialog.request
            );
            if !request_matches {
                app.notification = Some("The Wic device picker is stale.".into());
                return None;
            }
            replace_dialog(
                app,
                Dialog::WicWritePhrase(WicWritePhraseDialog {
                    request: dialog.request,
                    device: device.identity.clone(),
                    input: String::new(),
                    validation_error: None,
                }),
            );
        }
        Action::CancelWicDevicePicker => {
            if matches!(app.active_dialog(), Some(Dialog::WicDevicePicker(_))) {
                close_dialog(app);
            }
        }
        Action::AppendWicWritePhrase(character) => {
            if let Some(Dialog::WicWritePhrase(dialog)) = app.active_dialog_mut() {
                dialog.append(character);
            }
        }
        Action::BackspaceWicWritePhrase => {
            if let Some(Dialog::WicWritePhrase(dialog)) = app.active_dialog_mut() {
                dialog.backspace();
            }
        }
        Action::PreviewWicDeviceWrite => {
            let Some(Dialog::WicWritePhrase(dialog)) = app.active_dialog().cloned() else {
                return None;
            };
            match current_wic_write_preview(app, &dialog.request, &dialog.device, &dialog.input) {
                Ok(preview) => replace_dialog(app, Dialog::WicWriteConfirmation(preview)),
                Err(message) => {
                    if let Some(Dialog::WicWritePhrase(active)) = app.active_dialog_mut() {
                        active.validation_error = Some(message.clone());
                    }
                    app.notification = Some(message);
                }
            }
        }
        Action::CancelWicWritePhrase => {
            if matches!(app.active_dialog(), Some(Dialog::WicWritePhrase(_))) {
                close_dialog(app);
            }
        }
        Action::ConfirmWicDeviceWrite => {
            let Some(Dialog::WicWriteConfirmation(preview)) = app.active_dialog().cloned() else {
                return None;
            };
            let request = WicDeviceInventoryRequest {
                generation: match &app.wic_devices {
                    WicDeviceInventoryState::Available { request, .. }
                    | WicDeviceInventoryState::Partial { request, .. } => request.generation,
                    _ => {
                        app.notification = Some("The Wic device inventory is unavailable.".into());
                        return None;
                    }
                },
                image: preview.request.image.clone(),
            };
            let phrase = format!("WRITE {}", preview.request.device.path.display());
            let current =
                current_wic_write_preview(app, &request, &preview.request.device, &phrase);
            if current.as_ref() != Ok(&preview) {
                app.notification =
                    Some("The Wic device write preview is stale or no longer valid.".into());
                return None;
            }
            close_dialog(app);
            let effect = queue_wic_session(app, WicOperation::Write(preview.request));
            synchronize_focus(app);
            return effect;
        }
        Action::CancelWicWritePreview => {
            if matches!(app.active_dialog(), Some(Dialog::WicWriteConfirmation(_))) {
                close_dialog(app);
            }
        }
        Action::StartConfirmedWicCreate(preview) => {
            let Some(output_directory) = preview.request.output_directory.to_str() else {
                app.notification = Some("Wic output paths must be valid UTF-8.".into());
                return None;
            };
            let draft = WicCreateDraft {
                machine: preview.request.machine.clone(),
                image: preview.request.image.clone(),
                kickstart: preview.request.kickstart.clone(),
                output_directory: output_directory.into(),
                generate_bmap: preview.request.generate_bmap,
                compression: preview.request.compression,
            };
            if draft.preview(&app.wic_capability).as_ref() != Ok(&preview) {
                app.notification =
                    Some("The Wic creation preview is stale or no longer valid.".into());
                return None;
            }
            return queue_wic_session(app, WicOperation::Create(preview.request));
        }
        Action::WicSessionStarting { id, started_at } => {
            let Some(job_id) = wic_job_id(app, id) else {
                note_stale_wic_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Queued], |job| {
                    job.status = BackgroundJobStatus::Starting;
                    job.started_at = Some(started_at);
                });
        }
        Action::WicSessionRunning { id } => {
            let Some(job_id) = wic_job_id(app, id) else {
                note_stale_wic_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Starting], |job| {
                    job.status = BackgroundJobStatus::Running;
                });
        }
        Action::AppendWicSessionOutput {
            id,
            stream,
            line,
            truncated,
            timestamp,
        } => {
            let Some(job_id) = wic_job_id(app, id) else {
                note_stale_wic_event(app);
                return None;
            };
            app.background_jobs.append_output(
                job_id,
                BackgroundJobOutputEntry {
                    severity: if stream == WicOutputStream::Stderr {
                        Severity::Warning
                    } else {
                        Severity::Info
                    },
                    message: line,
                    source: if stream == WicOutputStream::Stderr {
                        BackgroundJobOutputSource::Stderr
                    } else {
                        BackgroundJobOutputSource::Stdout
                    },
                    truncated,
                    timestamp,
                },
            );
        }
        Action::CompleteWicSession {
            id,
            exit_code,
            outputs,
            limitations,
            finished_at,
        } => {
            let Some(session) = app.wic_session(id).cloned() else {
                note_stale_wic_event(app);
                return None;
            };
            let Some(job) = app.background_jobs.get(session.background_job_id) else {
                note_stale_wic_event(app);
                return None;
            };
            if job.status != BackgroundJobStatus::Running {
                note_stale_wic_event(app);
                return None;
            }
            let job_id = mutate_wic_session(app, id, |session| session.exit_code = Some(exit_code))
                .expect("session checked above");
            if exit_code == 0 {
                let artifacts = outputs
                    .iter()
                    .map(|output| output.identity.path.clone())
                    .collect();
                app.background_jobs
                    .update_if(job_id, &[BackgroundJobStatus::Running], |job| {
                        job.status = BackgroundJobStatus::Succeeded;
                        job.finished_at = Some(finished_at);
                        job.result = Some(BackgroundJobResult {
                            summary: "Wic operation completed".into(),
                            artifacts,
                        });
                    });
                if let WicOperation::Create(request) = session.operation {
                    let generation = app.wic_output_generation.wrapping_add(1).max(1);
                    app.wic_output_generation = generation;
                    let outputs = normalize_wic_outputs(&request.output_directory, outputs)
                        .unwrap_or_default();
                    let limitations = normalize_wic_limitations(limitations);
                    app.wic_outputs = if limitations.is_empty() {
                        WicOutputInventoryState::Available {
                            request: WicOutputInventoryRequest {
                                generation,
                                output_directory: request.output_directory,
                            },
                            outputs,
                        }
                    } else {
                        WicOutputInventoryState::Partial {
                            request: WicOutputInventoryRequest {
                                generation,
                                output_directory: request.output_directory,
                            },
                            outputs,
                            limitations,
                        }
                    };
                    reconcile_wic_output_selection(app);
                }
            } else {
                let message = format!("exit code {exit_code}");
                let _ = mutate_wic_session(app, id, |session| {
                    session.error_detail = Some(message.clone())
                });
                app.background_jobs
                    .update_if(job_id, &[BackgroundJobStatus::Running], |job| {
                        job.status = BackgroundJobStatus::Failed;
                        job.finished_at = Some(finished_at);
                        job.error = Some(BackgroundJobError {
                            summary: "Wic operation failed".into(),
                            detail: Some(message),
                        });
                    });
            }
        }
        Action::FailWicSession {
            id,
            message,
            exit_code,
            finished_at,
        } => {
            if !matches!(
                wic_job_id(app, id).and_then(|job_id| app.background_jobs.get(job_id)),
                Some(BackgroundJob {
                    status: BackgroundJobStatus::Queued
                        | BackgroundJobStatus::Starting
                        | BackgroundJobStatus::Running
                        | BackgroundJobStatus::Cancelling,
                    ..
                })
            ) {
                note_stale_wic_event(app);
                return None;
            }
            let Some(job_id) = mutate_wic_session(app, id, |session| {
                session.exit_code = exit_code;
                session.error_detail = Some(message.clone());
            }) else {
                note_stale_wic_event(app);
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
                        summary: "Wic operation failed".into(),
                        detail: Some(message),
                    });
                },
            );
        }
        Action::LoseWicSession {
            id,
            message,
            finished_at,
        } => {
            if !matches!(
                wic_job_id(app, id).and_then(|job_id| app.background_jobs.get(job_id)),
                Some(BackgroundJob {
                    status: BackgroundJobStatus::Starting
                        | BackgroundJobStatus::Running
                        | BackgroundJobStatus::Cancelling,
                    ..
                })
            ) {
                note_stale_wic_event(app);
                return None;
            }
            let Some(job_id) = mutate_wic_session(app, id, |session| {
                session.error_detail = Some(message.clone())
            }) else {
                note_stale_wic_event(app);
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
                    job.status = BackgroundJobStatus::Lost;
                    job.finished_at = Some(finished_at);
                    job.error = Some(BackgroundJobError {
                        summary: "Wic process was lost".into(),
                        detail: Some(message),
                    });
                },
            );
        }
        Action::ConfirmWicSessionCancellation {
            id,
            acknowledge_incomplete_device,
        } => {
            let Some(session) = app.wic_session(id) else {
                note_stale_wic_event(app);
                return None;
            };
            if matches!(session.operation, WicOperation::Write(_)) && !acknowledge_incomplete_device
            {
                app.notification = Some(
                    "A device write cancellation requires the incomplete-device warning.".into(),
                );
                return None;
            }
            let job_id = session.background_job_id;
            let before = app.background_jobs.get(job_id).map(|job| job.status);
            app.background_jobs.request_cancellation(job_id);
            if before == app.background_jobs.get(job_id).map(|job| job.status) {
                app.notification = Some("The Wic cancellation request was rejected.".into());
                return None;
            }
            if matches!(
                app.active_dialog(),
                Some(Dialog::WicCancellationConfirmation { id: candidate, .. })
                    if *candidate == id
            ) {
                close_dialog(app);
            }
            return Some(Effect::CancelWicSession(id));
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
