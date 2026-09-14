//! State transitions beginning with ConfirmWicDeviceSelection.
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
        Action::RejectWicSessionCancellation { id, message } => {
            let Some(job_id) = wic_job_id(app, id) else {
                note_stale_wic_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Cancelling], |job| {
                    job.status = BackgroundJobStatus::Running;
                });
            app.notification = Some(format!("Wic cancellation failed: {message}"));
        }
        Action::CancelWicSession {
            id,
            exit_code,
            finished_at,
        } => {
            if !matches!(
                wic_job_id(app, id).and_then(|job_id| app.background_jobs.get(job_id)),
                Some(BackgroundJob {
                    status: BackgroundJobStatus::Cancelling,
                    ..
                })
            ) {
                note_stale_wic_event(app);
                return None;
            }
            let is_device_write = matches!(
                app.wic_session(id).map(|session| &session.operation),
                Some(WicOperation::Write(_))
            );
            let Some(job_id) = mutate_wic_session(app, id, |session| session.exit_code = exit_code)
            else {
                note_stale_wic_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Cancelling], |job| {
                    job.status = BackgroundJobStatus::Cancelled;
                    job.finished_at = Some(finished_at);
                    if is_device_write {
                        job.error = Some(BackgroundJobError {
                            summary: "Wic device write cancelled".into(),
                            detail: Some("The target device may be incomplete.".into()),
                        });
                    }
                });
        }
        Action::BeginBuildTargetEdit => {
            let mut editor = PopupEditor::new(popup_toml_document(
                "target",
                app.build.target.as_deref().unwrap_or_default(),
                None,
            ));
            let _ = editor.select_toml_value("target");
            replace_dialog(app, Dialog::BuildTarget { editor, task: None });
        }
        Action::BeginBuildTargetTask(task) => {
            let mut editor = PopupEditor::new(popup_toml_document(
                "target",
                app.build.target.as_deref().unwrap_or_default(),
                None,
            ));
            let _ = editor.select_toml_value("target");
            replace_dialog(app, Dialog::BuildTarget { editor, task });
        }
        Action::ToggleBuildTargetEdit => {
            if let Some(Dialog::BuildTarget { editor, .. }) = app.active_dialog_mut() {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendBuildTarget(character) => {
            if let Some(Dialog::BuildTarget { editor, .. }) = app.active_dialog_mut()
                && editor.editing
            {
                editor.insert(&character.to_string());
            }
        }
        Action::BackspaceBuildTarget => {
            if let Some(Dialog::BuildTarget { editor, .. }) = app.active_dialog_mut()
                && editor.editing
            {
                editor.backspace();
            }
        }
        Action::CancelBuildTargetEdit => {
            if matches!(app.active_dialog(), Some(Dialog::BuildTarget { .. })) {
                close_dialog(app);
            }
        }
        Action::ConfirmBuildTarget => {
            if let Some(Dialog::BuildTarget { editor, task }) = app.active_dialog() {
                let input = match popup_toml_value(&editor.text, "target") {
                    Ok(value) => value,
                    Err(reason) => {
                        app.notification = Some(reason);
                        return None;
                    }
                };
                let request = BuildRequest {
                    targets: vec![input],
                    task: task.clone(),
                    force: false,
                };
                if let Err(error) = request.validate() {
                    app.notification = Some(error.to_string());
                } else {
                    replace_dialog(app, Dialog::RecipeTaskConfirmation(request));
                }
            }
        }
        Action::Start(r) => {
            if !app.build_environment.connected() {
                app.notification = Some("Configure and verify a BitBake environment first".into());
            } else if let Err(e) = r.validate() {
                app.notification = Some(e.to_string())
            } else {
                prepare_build(app, r.targets.first().cloned());
                return Some(Effect::Start(r));
            }
        }
        Action::QueueBackgroundJob(spec) => app.background_jobs.queue(spec),
        Action::StartBackgroundJob { id, started_at } => {
            app.background_jobs
                .update_if(id, &[BackgroundJobStatus::Queued], |job| {
                    job.status = BackgroundJobStatus::Starting;
                    job.started_at = Some(started_at);
                })
        }
        Action::RunBackgroundJob { id } => {
            app.background_jobs
                .update_if(id, &[BackgroundJobStatus::Starting], |job| {
                    job.status = BackgroundJobStatus::Running
                })
        }
        Action::UpdateBackgroundJobProgress { id, progress } => {
            if progress.is_valid() {
                app.background_jobs
                    .update_if(id, &[BackgroundJobStatus::Running], |job| {
                        job.progress = progress
                    });
            } else {
                app.background_jobs.ignored_transitions += 1;
            }
        }
        Action::AppendBackgroundJobOutput { id, entry } => {
            app.background_jobs.append_output(id, entry);
        }
        Action::RequestBackgroundJobCancellation { id } => {
            app.background_jobs.request_cancellation(id);
        }
        Action::RejectBackgroundJobCancellation { id } => {
            app.background_jobs
                .update_if(id, &[BackgroundJobStatus::Cancelling], |job| {
                    job.status = BackgroundJobStatus::Running
                })
        }
        Action::SucceedBackgroundJob {
            id,
            result,
            finished_at,
        } => app
            .background_jobs
            .update_if(id, &[BackgroundJobStatus::Running], |job| {
                job.status = BackgroundJobStatus::Succeeded;
                job.finished_at = Some(finished_at);
                job.result = Some(result);
            }),
        Action::FailBackgroundJob {
            id,
            error,
            finished_at,
        } => app.background_jobs.update_if(
            id,
            &[
                BackgroundJobStatus::Starting,
                BackgroundJobStatus::Running,
                BackgroundJobStatus::Cancelling,
            ],
            |job| {
                job.status = BackgroundJobStatus::Failed;
                job.finished_at = Some(finished_at);
                job.error = Some(error);
            },
        ),
        Action::CancelBackgroundJob { id, finished_at } => {
            app.background_jobs
                .update_if(id, &[BackgroundJobStatus::Cancelling], |job| {
                    job.status = BackgroundJobStatus::Cancelled;
                    job.finished_at = Some(finished_at);
                })
        }
        Action::LoseBackgroundJob {
            id,
            error,
            finished_at,
        } => app.background_jobs.update_if(
            id,
            &[
                BackgroundJobStatus::Starting,
                BackgroundJobStatus::Running,
                BackgroundJobStatus::Cancelling,
            ],
            |job| {
                job.status = BackgroundJobStatus::Lost;
                job.finished_at = Some(finished_at);
                job.error = Some(error);
            },
        ),
        Action::BuildRequested { target } => prepare_build(app, target),
        Action::BuildStarted => {
            app.build.status = BuildStatus::Running;
            app.build.started = Some(SystemTime::now());
            app.build.parse_current = None;
            app.build.parse_total = None;
        }
        Action::TaskStats(stats) => {
            mark_build_running_from_task_activity(app);
            app.build.completed = app.build.completed.max(stats.completed);
            app.build.total = (stats.total > 0).then_some(stats.total);
            app.invalidate_task_projection();
        }
        Action::ParseProgress { current, total } => {
            if matches!(
                app.build.status,
                BuildStatus::LoadingWorkspace | BuildStatus::Parsing | BuildStatus::Running
            ) && app.tasks.is_empty()
                && app.build.completed == 0
            {
                app.build.status = BuildStatus::Parsing;
                app.build.parse_current = current;
                app.build.parse_total = total;
            }
        }
        Action::TaskStarted(t) => {
            apply_task_event(app, TaskEvent::Started(t));
            clamp_task_selection(app);
        }
        Action::TaskQueued(t) => {
            apply_task_event(app, TaskEvent::Queued(t));
            clamp_task_selection(app);
        }
        Action::TaskProgress { id, progress } => {
            app.task_progress_events = app.task_progress_events.saturating_add(1);
            apply_task_event(app, TaskEvent::Progress { id, progress });
        }
        Action::TaskCompleted { id, success } => {
            apply_task_event(app, TaskEvent::Completed { id, success });
            clamp_task_selection(app);
        }
        Action::TaskEvents(events) => apply_task_batch(app, events),
        Action::ScrollBuildTasks { delta } => {
            let task_count = app.visible_task_rows().len();
            app.task_progress_scroll = shifted_index(app.task_progress_scroll, delta, task_count);
        }
        Action::CycleTaskStateFilter => {
            app.task_filters.state = app.task_filters.state.next();
            clamp_task_selection(app);
        }
        Action::CycleTaskFilterField => {
            app.task_filter_field = app.task_filter_field.next();
        }
        Action::BeginTaskFilterEdit => {
            app.task_filter_editing = true;
        }
        Action::AppendTaskFilter(character) => {
            if app.task_filter_editing {
                match app.task_filter_field {
                    TaskFilterField::Recipe => app.task_filters.recipe.push(character),
                    TaskFilterField::Task => app.task_filters.task.push(character),
                    TaskFilterField::Worker => app.task_filters.worker.push(character),
                }
                clamp_task_selection(app);
            }
        }
        Action::BackspaceTaskFilter => {
            if app.task_filter_editing {
                match app.task_filter_field {
                    TaskFilterField::Recipe => app.task_filters.recipe.pop(),
                    TaskFilterField::Task => app.task_filters.task.pop(),
                    TaskFilterField::Worker => app.task_filters.worker.pop(),
                };
                clamp_task_selection(app);
            }
        }
        Action::FinishTaskFilterEdit => {
            app.task_filter_editing = false;
        }
        Action::CycleTaskDurationFilter => {
            app.task_filters.minimum_duration = match app.task_filters.minimum_duration {
                None => Some(Duration::from_secs(1)),
                Some(duration) if duration == Duration::from_secs(1) => {
                    Some(Duration::from_secs(10))
                }
                Some(duration) if duration == Duration::from_secs(10) => {
                    Some(Duration::from_secs(60))
                }
                Some(_) => None,
            };
            clamp_task_selection(app);
        }
        Action::Log(entry) => insert_log_entry(app, entry),
        Action::Logs(entries) => insert_log_batch(app, entries),
        Action::BuildCompleted { success, exit_code } => {
            archive_unfinished_tasks(app, TaskState::Lost, Some("build ended"));
            app.build.status = if success {
                BuildStatus::Completed
            } else {
                BuildStatus::Failed
            };
            if !success {
                app.build.errors = app.build.errors.max(1);
            }
            if success && let Some(total) = app.build.total {
                app.build.completed = total;
            }
            insert_system_log(
                app,
                if success {
                    Severity::Info
                } else {
                    Severity::Error
                },
                format!(
                    "Build {} with exit code {}",
                    if success { "completed" } else { "failed" },
                    exit_code.map_or_else(|| "unknown".into(), |code| code.to_string())
                ),
            );
            app.build.exit_code = exit_code;
            app.build_history.push_back(BuildRecord {
                target: app.build.target.clone(),
                success,
                exit_code,
                elapsed: app.elapsed(),
                completed_tasks: app.build.completed,
                warnings: app.build.warnings,
                errors: app.build.errors,
            });
            if app.build_history.len() > MAX_BUILD_HISTORY {
                app.build_history.pop_front();
            }
            app.build_history_selection = 0;
            clamp_task_selection(app);
            enqueue_build_completion(app);
            app.notification = Some(if success {
                if app.build.warnings > 0 {
                    format!(
                        "Build completed with {} warning(s). Open Errors to investigate.",
                        app.build.warnings
                    )
                } else {
                    "Build completed successfully with no errors.".into()
                }
            } else {
                format!(
                    "Build failed with {} error(s). Press Enter to open Errors.",
                    app.build.errors
                )
            });
        }
        Action::BuildAuthorityLost { message } => {
            if matches!(
                app.build.status,
                BuildStatus::LoadingWorkspace
                    | BuildStatus::Parsing
                    | BuildStatus::Running
                    | BuildStatus::Cancelling
            ) {
                archive_unfinished_tasks(app, TaskState::Lost, Some("build authority lost"));
                app.build.status = BuildStatus::Lost;
                app.build.exit_code = None;
                insert_system_log(
                    app,
                    Severity::Warning,
                    format!("Build authority lost: {message}"),
                );
                app.notification = Some(format!(
                    "Build authority lost: {message}. Reconnecting to the daemon."
                ));
            }
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
