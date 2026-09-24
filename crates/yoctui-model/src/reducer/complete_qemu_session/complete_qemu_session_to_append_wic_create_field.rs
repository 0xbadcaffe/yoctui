use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::CompleteQemuSession {
            id,
            exit_code,
            finished_at,
        } => {
            if !matches!(
                qemu_job_id(app, id).and_then(|job_id| app.background_jobs.get(job_id)),
                Some(BackgroundJob {
                    status: BackgroundJobStatus::Running,
                    ..
                })
            ) {
                note_stale_qemu_event(app);
                return None;
            }
            let Some(job_id) =
                mutate_qemu_session(app, id, |session| session.exit_code = Some(exit_code))
            else {
                note_stale_qemu_event(app);
                return None;
            };
            if exit_code == 0 {
                app.background_jobs
                    .update_if(job_id, &[BackgroundJobStatus::Running], |job| {
                        job.status = BackgroundJobStatus::Succeeded;
                        job.finished_at = Some(finished_at);
                        job.result = Some(BackgroundJobResult {
                            summary: "runqemu exited successfully".into(),
                            artifacts: Vec::new(),
                        });
                    });
            } else {
                let detail = format!("exit code {exit_code}");
                let _ = mutate_qemu_session(app, id, |session| {
                    session.error_detail = Some(detail.clone())
                });
                app.background_jobs
                    .update_if(job_id, &[BackgroundJobStatus::Running], |job| {
                        job.status = BackgroundJobStatus::Failed;
                        job.finished_at = Some(finished_at);
                        job.error = Some(BackgroundJobError {
                            summary: "runqemu failed".into(),
                            detail: Some(detail),
                        });
                    });
            }
        }
        Action::FailQemuSession {
            id,
            message,
            exit_code,
            finished_at,
        } => {
            if !matches!(
                qemu_job_id(app, id).and_then(|job_id| app.background_jobs.get(job_id)),
                Some(BackgroundJob {
                    status: BackgroundJobStatus::Queued
                        | BackgroundJobStatus::Starting
                        | BackgroundJobStatus::Running
                        | BackgroundJobStatus::Cancelling,
                    ..
                })
            ) {
                note_stale_qemu_event(app);
                return None;
            }
            let Some(job_id) = mutate_qemu_session(app, id, |session| {
                session.exit_code = exit_code;
                session.error_detail = Some(message.clone());
            }) else {
                note_stale_qemu_event(app);
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
                        summary: "runqemu failed".into(),
                        detail: Some(message),
                    });
                },
            );
        }
        Action::LoseQemuSession {
            id,
            message,
            finished_at,
        } => {
            if !matches!(
                qemu_job_id(app, id).and_then(|job_id| app.background_jobs.get(job_id)),
                Some(BackgroundJob {
                    status: BackgroundJobStatus::Starting
                        | BackgroundJobStatus::Running
                        | BackgroundJobStatus::Cancelling,
                    ..
                })
            ) {
                note_stale_qemu_event(app);
                return None;
            }
            let Some(job_id) = mutate_qemu_session(app, id, |session| {
                session.error_detail = Some(message.clone())
            }) else {
                note_stale_qemu_event(app);
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
                        summary: "runqemu process was lost".into(),
                        detail: Some(message),
                    });
                },
            );
        }
        Action::BeginQemuSessionCancellation { id } => {
            let Some(session) = app.qemu_session(id) else {
                app.notification = Some("The runqemu session no longer exists.".into());
                return None;
            };
            let cancellable = app
                .background_jobs
                .get(session.background_job_id)
                .is_some_and(|job| {
                    job.cancellation_supported
                        && matches!(
                            job.status,
                            BackgroundJobStatus::Queued
                                | BackgroundJobStatus::Starting
                                | BackgroundJobStatus::Running
                        )
                });
            if cancellable {
                open_dialog(app, Dialog::QemuCancellationConfirmation(id));
            } else {
                app.notification = Some("The runqemu session cannot be cancelled.".into());
            }
        }
        Action::BeginActiveQemuSessionCancellation => {
            let Some(id) = app.active_qemu_session().map(|session| session.id) else {
                app.notification = Some("No managed runqemu session is active.".into());
                return None;
            };
            let _ = update(app, Action::BeginQemuSessionCancellation { id });
        }
        Action::ConfirmQemuSessionCancellation => {
            let Some(Dialog::QemuCancellationConfirmation(id)) = app.active_dialog().cloned()
            else {
                app.notification = Some("No runqemu cancellation is awaiting confirmation.".into());
                return None;
            };
            let Some(job_id) = qemu_job_id(app, id) else {
                note_stale_qemu_event(app);
                return None;
            };
            let before = app.background_jobs.get(job_id).map(|job| job.status);
            app.background_jobs.request_cancellation(job_id);
            close_dialog(app);
            if before
                == Some(
                    app.background_jobs
                        .get(job_id)
                        .map_or(BackgroundJobStatus::Lost, |job| job.status),
                )
            {
                app.notification = Some("The runqemu cancellation request was rejected.".into());
                return None;
            }
            return Some(Effect::CancelQemuSession(id));
        }
        Action::CancelQemuSessionCancellation => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::QemuCancellationConfirmation(_))
            ) {
                close_dialog(app);
            }
        }
        Action::RejectQemuSessionCancellation { id, message } => {
            let Some(job_id) = qemu_job_id(app, id) else {
                note_stale_qemu_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Cancelling], |job| {
                    job.status = BackgroundJobStatus::Running;
                });
            app.notification = Some(format!("runqemu cancellation failed: {message}"));
        }
        Action::CancelQemuSession {
            id,
            exit_code,
            finished_at,
        } => {
            if !matches!(
                qemu_job_id(app, id).and_then(|job_id| app.background_jobs.get(job_id)),
                Some(BackgroundJob {
                    status: BackgroundJobStatus::Cancelling,
                    ..
                })
            ) {
                note_stale_qemu_event(app);
                return None;
            }
            let Some(job_id) =
                mutate_qemu_session(app, id, |session| session.exit_code = exit_code)
            else {
                note_stale_qemu_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Cancelling], |job| {
                    job.status = BackgroundJobStatus::Cancelled;
                    job.finished_at = Some(finished_at);
                });
        }
        Action::InspectWicCapability => {
            app.wic_capability = WicCapability::NotInspected;
            return Some(Effect::InspectWicCapability);
        }
        Action::WicCapabilityLoaded(capability) => {
            app.wic_capability = normalize_wic_capability(capability);
            if app.pending_wic_create {
                app.pending_wic_create = false;
                if let WicCapability::Available { image_targets, .. } = &app.wic_capability
                    && !image_targets.is_empty()
                    && !app
                        .selected_image_artifact()
                        .is_some_and(|artifact| image_targets.contains(&artifact.identity.image))
                {
                    app.image_artifact_selection = app
                        .image_artifacts
                        .artifacts()
                        .unwrap_or_default()
                        .iter()
                        .find(|artifact| image_targets.contains(&artifact.identity.image))
                        .map(|artifact| artifact.identity.clone());
                }
                return update(app, Action::BeginSelectedWicCreate);
            }
        }
        Action::BeginSelectedWicCreate => {
            if matches!(app.wic_capability, WicCapability::NotInspected) {
                app.pending_wic_create = true;
                return Some(Effect::InspectWicCapability);
            }
            if let WicCapability::Available { image_targets, .. } = &app.wic_capability
                && !image_targets.is_empty()
                && !app
                    .selected_image_artifact()
                    .is_some_and(|artifact| image_targets.contains(&artifact.identity.image))
            {
                app.image_artifact_selection = app
                    .image_artifacts
                    .artifacts()
                    .unwrap_or_default()
                    .iter()
                    .find(|artifact| image_targets.contains(&artifact.identity.image))
                    .map(|artifact| artifact.identity.clone());
            }
            if let Some(reason) = app.wic_create_unavailable_reason() {
                app.notification = Some(reason);
                return None;
            }
            let artifact = app
                .selected_image_artifact()
                .cloned()
                .expect("checked above");
            let WicCapability::Available { kickstarts, .. } = &app.wic_capability else {
                unreachable!("checked above")
            };
            let kickstart = app
                .workspace
                .variables
                .get("WKS_FILE")
                .and_then(|configured| {
                    let configured_path = Path::new(configured);
                    kickstarts.iter().find(|kickstart| {
                        kickstart.identity.name == *configured
                            || kickstart.identity.path.as_deref() == Some(configured_path)
                    })
                })
                .unwrap_or(&kickstarts[0])
                .identity
                .clone();
            let output_directory = artifact
                .identity
                .path
                .parent()
                .unwrap_or(Path::new("/"))
                .display()
                .to_string();
            let mut editor = PopupEditor::new(format!(
                "# machine is authoritative and read-only\nmachine = \"{}\"\nimage = \"{}\"\nkickstart = \"{}\"\noutput_directory = \"{}\"\ngenerate_bmap = true\ncompression = \"none\"\n",
                artifact.identity.machine,
                artifact.identity.image,
                kickstart.name,
                output_directory,
            ));
            let _ = editor.select_toml_value("output_directory");
            open_dialog(
                app,
                Dialog::WicCreateTomlEditor {
                    editor,
                    validation_error: None,
                },
            );
        }
        Action::ToggleWicCreateTomlEditor => {
            if let Some(Dialog::WicCreateTomlEditor { editor, .. }) = app.active_dialog_mut() {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendWicCreateTomlEditor(character) => {
            if let Some(Dialog::WicCreateTomlEditor {
                editor,
                validation_error,
            }) = app.active_dialog_mut()
                && editor.editing
                && !character.is_control()
            {
                editor.insert(&character.to_string());
                *validation_error = None;
            }
        }
        Action::BackspaceWicCreateTomlEditor => {
            if let Some(Dialog::WicCreateTomlEditor {
                editor,
                validation_error,
            }) = app.active_dialog_mut()
                && editor.editing
            {
                editor.backspace();
                *validation_error = None;
            }
        }
        Action::SelectWicCreateField { delta } => {
            if let Some(Dialog::WicCreate(dialog)) = app.active_dialog_mut()
                && !dialog.editing
            {
                dialog.selected_field = dialog.selected_field.shifted(delta);
            }
        }
        Action::ActivateWicCreateField => {
            let capability = app.wic_capability.clone();
            if let Some(Dialog::WicCreate(dialog)) = app.active_dialog_mut() {
                if dialog.selected_field.is_read_only() {
                    app.notification = Some("Wic machine identity is read-only.".into());
                } else if dialog.selected_field == WicCreateField::OutputDirectory {
                    dialog.editing = true;
                    dialog.validation_error = None;
                } else if dialog.cycle_choice(&capability, false) {
                    dialog.validation_error = None;
                }
            }
        }
        Action::CycleWicCreateChoice { backwards } => {
            let capability = app.wic_capability.clone();
            if let Some(Dialog::WicCreate(dialog)) = app.active_dialog_mut()
                && !dialog.editing
                && dialog.cycle_choice(&capability, backwards)
            {
                dialog.validation_error = None;
            }
        }
        Action::AppendWicCreateField(character) => {
            if let Some(Dialog::WicCreate(dialog)) = app.active_dialog_mut()
                && dialog.editing
                && !character.is_control()
                && let Some((input, maximum)) = dialog.selected_text_mut()
                && input.len() + character.len_utf8() <= maximum
            {
                input.push(character);
                dialog.validation_error = None;
            }
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
