//! State transitions beginning with CompleteQemuSession.
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
        }
        Action::BeginSelectedWicCreate => {
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
        Action::BackspaceWicCreateField => {
            if let Some(Dialog::WicCreate(dialog)) = app.active_dialog_mut()
                && dialog.editing
                && let Some((input, _)) = dialog.selected_text_mut()
            {
                input.pop();
                dialog.validation_error = None;
            }
        }
        Action::FinishWicCreateFieldEdit => {
            if let Some(Dialog::WicCreate(dialog)) = app.active_dialog_mut() {
                dialog.editing = false;
            }
        }
        Action::PreviewWicCreate => {
            if let Some(Dialog::WicCreateTomlEditor { editor, .. }) = app.active_dialog().cloned() {
                let result = (|| {
                    let fields = popup_toml_fields(&editor.text)?;
                    let machine = fields.get("machine").cloned().ok_or("Missing `machine`.")?;
                    let image = fields.get("image").cloned().ok_or("Missing `image`.")?;
                    let kickstart_name = fields
                        .get("kickstart")
                        .cloned()
                        .ok_or("Missing `kickstart`.")?;
                    let output_directory = fields
                        .get("output_directory")
                        .cloned()
                        .ok_or("Missing `output_directory`.")?;
                    let generate_bmap = match fields.get("generate_bmap").map(String::as_str) {
                        Some("true") => true,
                        Some("false") => false,
                        _ => return Err("`generate_bmap` must be true or false.".to_owned()),
                    };
                    let compression = match fields.get("compression").map(String::as_str) {
                        Some("none") => WicCompression::None,
                        Some("gzip") => WicCompression::Gzip,
                        Some("bzip2") => WicCompression::Bzip2,
                        Some("xz") => WicCompression::Xz,
                        _ => {
                            return Err(
                                "`compression` must be none, gzip, bzip2, or xz.".to_owned()
                            );
                        }
                    };
                    let expected_machine = app
                        .selected_image_artifact()
                        .map(|artifact| artifact.identity.machine.as_str())
                        .ok_or_else(|| "The selected image artifact is unavailable.".to_owned())?;
                    if machine != expected_machine {
                        return Err(
                            "`machine` is authoritative and cannot differ from the selected image."
                                .to_owned(),
                        );
                    }
                    let WicCapability::Available { kickstarts, .. } = &app.wic_capability else {
                        return Err("Wic capability is not available.".to_owned());
                    };
                    let kickstart = kickstarts
                        .iter()
                        .find(|candidate| candidate.identity.name == kickstart_name)
                        .map(|candidate| candidate.identity.clone())
                        .ok_or_else(|| "The selected kickstart is unavailable.".to_owned())?;
                    WicCreateDraft {
                        machine,
                        image,
                        kickstart,
                        output_directory,
                        generate_bmap,
                        compression,
                    }
                    .preview(&app.wic_capability)
                    .map_err(str::to_owned)
                })();
                match result {
                    Ok(preview) => replace_dialog(app, Dialog::WicCreateConfirmation(preview)),
                    Err(message) => {
                        if let Some(Dialog::WicCreateTomlEditor {
                            validation_error, ..
                        }) = app.active_dialog_mut()
                        {
                            *validation_error = Some(message.clone());
                        }
                        app.notification = Some(message);
                    }
                }
                return None;
            }
            let Some(Dialog::WicCreate(dialog)) = app.active_dialog().cloned() else {
                app.notification = Some("No Wic creation draft is active.".into());
                return None;
            };
            match dialog.draft.preview(&app.wic_capability) {
                Ok(preview) => replace_dialog(app, Dialog::WicCreateConfirmation(preview)),
                Err(message) => {
                    if let Some(Dialog::WicCreate(dialog)) = app.active_dialog_mut() {
                        dialog.validation_error = Some(message.into());
                    }
                    app.notification = Some(message.into());
                }
            }
        }
        Action::CancelWicCreate => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::WicCreate(_) | Dialog::WicCreateTomlEditor { .. })
            ) {
                close_dialog(app);
            }
        }
        Action::CancelWicCreatePreview => {
            if matches!(app.active_dialog(), Some(Dialog::WicCreateConfirmation(_))) {
                close_dialog(app);
            }
        }
        Action::ConfirmWicCreate => {
            let Some(Dialog::WicCreateConfirmation(preview)) = app.active_dialog().cloned() else {
                app.notification = Some("No Wic creation is awaiting confirmation.".into());
                return None;
            };
            close_dialog(app);
            return update(app, Action::StartConfirmedWicCreate(preview));
        }
        Action::SelectWicOutput { delta } => {
            let rows = app.wic_output_rows();
            if rows.is_empty() {
                app.wic_output_selection = None;
                return None;
            }
            let current = app
                .wic_output_selection
                .as_ref()
                .and_then(|selected| rows.iter().position(|row| &row.identity == selected))
                .unwrap_or(0);
            let next = if delta.is_negative() {
                current.saturating_sub(delta.unsigned_abs())
            } else {
                current.saturating_add(delta as usize).min(rows.len() - 1)
            };
            app.wic_output_selection = Some(rows[next].identity.clone());
        }
        Action::OpenSelectedWicOutput => {
            let Some(output) = app.selected_wic_output() else {
                app.notification = Some("Select a generated Wic output first.".into());
                return None;
            };
            return Some(Effect::OpenInEditor(output.identity.path.clone()));
        }
        Action::BeginActiveWicSessionCancellation => {
            let Some((id, incomplete_device_warning)) = app.active_wic_session().map(|session| {
                (
                    session.id,
                    matches!(session.operation, WicOperation::Write(_)),
                )
            }) else {
                app.notification = Some("No managed Wic operation is active.".into());
                return None;
            };
            open_dialog(
                app,
                Dialog::WicCancellationConfirmation {
                    id,
                    incomplete_device_warning,
                },
            );
        }
        Action::BeginActiveImageRuntimeCancellation => {
            if app.active_wic_session().is_some() {
                return update(app, Action::BeginActiveWicSessionCancellation);
            }
            return update(app, Action::BeginActiveQemuSessionCancellation);
        }
        Action::CancelWicSessionCancellation => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::WicCancellationConfirmation { .. })
            ) {
                close_dialog(app);
            }
        }
        Action::BeginWicOutputInventory(request) => {
            if let Err(message) = request.validate() {
                app.notification = Some(format!("Wic outputs are unavailable: {message}."));
                return None;
            }
            app.wic_output_generation = app.wic_output_generation.max(request.generation);
            app.wic_outputs = WicOutputInventoryState::Loading {
                request: request.clone(),
            };
            return Some(Effect::GetWicOutputs(request));
        }
        Action::WicOutputInventoryLoaded {
            request,
            outputs,
            limitations,
        } => {
            if !matches!(
                &app.wic_outputs,
                WicOutputInventoryState::Loading { request: active }
                    if active == &request
            ) {
                note_stale_wic_event(app);
                return None;
            }
            match normalize_wic_outputs(&request.output_directory, outputs) {
                Ok(outputs) => {
                    let limitations = normalize_wic_limitations(limitations);
                    app.wic_outputs = if limitations.is_empty() {
                        WicOutputInventoryState::Available { request, outputs }
                    } else {
                        WicOutputInventoryState::Partial {
                            request,
                            outputs,
                            limitations,
                        }
                    };
                }
                Err(message) => {
                    app.wic_outputs = WicOutputInventoryState::Failed {
                        request,
                        message: message.into(),
                    };
                }
            }
            reconcile_wic_output_selection(app);
        }
        Action::WicOutputInventoryFailed { request, message } => {
            if !matches!(
                &app.wic_outputs,
                WicOutputInventoryState::Loading { request: active }
                    if active == &request
            ) {
                note_stale_wic_event(app);
                return None;
            }
            app.wic_outputs = WicOutputInventoryState::Failed { request, message };
        }
        Action::BeginSelectedWicDeviceWrite => {
            if let Some(reason) = app.wic_device_write_unavailable_reason() {
                app.notification = Some(reason);
                return None;
            }
            let image = app
                .selected_wic_write_image()
                .expect("availability checked above");
            app.wic_device_generation = app.wic_device_generation.wrapping_add(1).max(1);
            let request = WicDeviceInventoryRequest {
                generation: app.wic_device_generation,
                image,
            };
            if let Err(message) = request.validate() {
                app.notification = Some(format!("Wic devices are unavailable: {message}."));
                return None;
            }
            let preserve_selection = matches!(
                &app.wic_devices,
                WicDeviceInventoryState::Loading { request: active }
                    | WicDeviceInventoryState::Available {
                        request: active,
                        ..
                    }
                    | WicDeviceInventoryState::Partial {
                        request: active,
                        ..
                    }
                    | WicDeviceInventoryState::Failed {
                        request: active,
                        ..
                    } if active.image == request.image
            );
            if !preserve_selection {
                app.wic_device_selection = None;
            }
            app.wic_devices = WicDeviceInventoryState::Loading {
                request: request.clone(),
            };
            open_dialog(
                app,
                Dialog::WicDevicePicker(WicDevicePickerDialog {
                    request: request.clone(),
                }),
            );
            synchronize_focus(app);
            return Some(Effect::GetWicDevices(request));
        }
        Action::BeginWicDeviceInventory(request) => {
            if let Err(message) = request.validate() {
                app.notification = Some(format!("Wic devices are unavailable: {message}."));
                return None;
            }
            app.wic_device_generation = app.wic_device_generation.max(request.generation);
            app.wic_devices = WicDeviceInventoryState::Loading {
                request: request.clone(),
            };
            return Some(Effect::GetWicDevices(request));
        }
        Action::WicDeviceInventoryLoaded {
            request,
            devices,
            limitations,
        } => {
            if !matches!(
                &app.wic_devices,
                WicDeviceInventoryState::Loading { request: active }
                    if active == &request
            ) {
                note_stale_wic_event(app);
                return None;
            }
            let devices = normalize_wic_devices(devices);
            let limitations = normalize_wic_limitations(limitations);
            app.wic_devices = if limitations.is_empty() {
                WicDeviceInventoryState::Available { request, devices }
            } else {
                WicDeviceInventoryState::Partial {
                    request,
                    devices,
                    limitations,
                }
            };
            reconcile_wic_device_selection(app);
        }
        Action::WicDeviceInventoryFailed { request, message } => {
            if !matches!(
                &app.wic_devices,
                WicDeviceInventoryState::Loading { request: active }
                    if active == &request
            ) {
                note_stale_wic_event(app);
                return None;
            }
            app.wic_devices = WicDeviceInventoryState::Failed { request, message };
            app.wic_device_selection = None;
        }
        Action::SelectWicDevice { delta } => {
            let Some(Dialog::WicDevicePicker(dialog)) = app.active_dialog() else {
                return None;
            };
            let request_matches = matches!(
                &app.wic_devices,
                WicDeviceInventoryState::Available { request, .. }
                    | WicDeviceInventoryState::Partial { request, .. }
                    if request == &dialog.request
            );
            if !request_matches {
                return None;
            }
            let rows = app.wic_device_rows();
            if rows.is_empty() {
                app.wic_device_selection = None;
                return None;
            }
            let current = app
                .wic_device_selection
                .as_ref()
                .and_then(|selected| rows.iter().position(|row| &row.identity == selected))
                .unwrap_or(0);
            let next = if delta.is_negative() {
                current.saturating_sub(delta.unsigned_abs())
            } else {
                current.saturating_add(delta as usize).min(rows.len() - 1)
            };
            app.wic_device_selection = Some(rows[next].identity.clone());
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
