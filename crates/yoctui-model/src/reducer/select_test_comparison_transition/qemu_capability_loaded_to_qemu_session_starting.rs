use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::QemuCapabilityLoaded(capability) => {
            app.qemu_capability = capability;
            if app.pending_qemu_launch {
                app.pending_qemu_launch = false;
                if let QemuCapability::Available {
                    compatible_images, ..
                } = &app.qemu_capability
                    && !compatible_images.is_empty()
                    && !app
                        .image_artifact_selection
                        .as_ref()
                        .is_some_and(|selected| compatible_images.contains(selected))
                {
                    app.image_artifact_selection = compatible_images.first().cloned();
                }
                return update(app, Action::BeginSelectedQemuLaunch);
            }
        }
        Action::SshClientCapabilityDetected(capability) => {
            app.ssh_client_capability = capability;
        }
        Action::BeginSelectedImageConsole => {
            let Some(artifact) = app.selected_image_artifact().cloned() else {
                app.notification = Some("Select a deployed image artifact first.".into());
                return None;
            };
            let mut draft = ImageConsoleDraft::for_artifact(artifact.identity, artifact.kind);
            let qemu_available = matches!(
                draft.artifact_kind,
                ImageArtifactKind::RootFilesystem | ImageArtifactKind::Wic
            ) && app.qemu_capability.executable_for(&draft.image).is_ok();
            if !qemu_available && app.ssh_client_capability.executable().is_ok() {
                draft.mode = ImageConsoleMode::Ssh;
            }
            open_dialog(app, Dialog::ImageConsole(ImageConsoleDialog::new(draft)));
        }
        Action::SelectImageConsoleField { delta } => {
            if let Some(Dialog::ImageConsole(dialog)) = app.active_dialog_mut() {
                dialog.shift_field(delta);
                dialog.validation_error = None;
            }
        }
        Action::CycleImageConsoleChoice { backwards } => {
            if let Some(Dialog::ImageConsole(dialog)) = app.active_dialog_mut()
                && dialog.cycle_choice(backwards)
            {
                dialog.validation_error = None;
            }
        }
        Action::AppendImageConsoleField(character) => {
            if character.is_control() {
                return None;
            }
            if let Some(Dialog::ImageConsole(dialog)) = app.active_dialog_mut()
                && let Some((text, maximum)) = dialog.selected_text_mut()
                && text.len().saturating_add(character.len_utf8()) <= maximum
            {
                text.push(character);
                dialog.validation_error = None;
            }
        }
        Action::BackspaceImageConsoleField => {
            if let Some(Dialog::ImageConsole(dialog)) = app.active_dialog_mut()
                && let Some((text, _)) = dialog.selected_text_mut()
            {
                text.pop();
                dialog.validation_error = None;
            }
        }
        Action::ConfirmImageConsole => {
            let Some(Dialog::ImageConsole(dialog)) = app.active_dialog().cloned() else {
                app.notification = Some("No Image Console request is active.".into());
                return None;
            };
            if !app
                .selected_image_artifact()
                .is_some_and(|artifact| artifact.identity == dialog.draft.image)
            {
                if let Some(Dialog::ImageConsole(current)) = app.active_dialog_mut() {
                    current.validation_error =
                        Some("The selected image artifact is stale; close and reopen.".into());
                }
                return None;
            }
            let Some(cwd) = app.workspace.build_dir.clone() else {
                if let Some(Dialog::ImageConsole(current)) = app.active_dialog_mut() {
                    current.validation_error =
                        Some("The daemon build directory is unavailable.".into());
                }
                return None;
            };
            match dialog
                .draft
                .preview(&app.qemu_capability, &app.ssh_client_capability)
            {
                Ok(preview) => {
                    close_dialog(app);
                    app.screen = Screen::TerminalSessions;
                    app.focus = FocusTarget::Workspace;
                    app.focus_return = None;
                    app.pty_selection = app.daemon.pty_sessions.len();
                    app.notification = Some(
                        "Image Console requested; press o when the session appears to take writer control."
                            .into(),
                    );
                    return Some(Effect::Terminal(TerminalEffect::Create {
                        name: preview.name,
                        kind: preview.kind,
                        cwd,
                        program: preview.program,
                        arguments: preview.arguments,
                    }));
                }
                Err(message) => {
                    if let Some(Dialog::ImageConsole(current)) = app.active_dialog_mut() {
                        current.validation_error = Some(message);
                    }
                }
            }
        }
        Action::CancelImageConsole => {
            if matches!(app.active_dialog(), Some(Dialog::ImageConsole(_))) {
                close_dialog(app);
            }
        }
        Action::BeginSelectedQemuLaunch => {
            if matches!(app.qemu_capability, QemuCapability::NotInspected) {
                app.pending_qemu_launch = true;
                return Some(Effect::InspectQemuCapability);
            }
            if let QemuCapability::Available {
                compatible_images, ..
            } = &app.qemu_capability
                && !compatible_images.is_empty()
                && !app
                    .image_artifact_selection
                    .as_ref()
                    .is_some_and(|selected| compatible_images.contains(selected))
            {
                app.image_artifact_selection = compatible_images.first().cloned();
            }
            if let Some(reason) = app.qemu_launch_unavailable_reason() {
                app.notification = Some(reason);
                return None;
            }
            let artifact = app.selected_image_artifact().cloned()?;
            let draft = QemuLaunchDraft::for_artifact(artifact.identity, artifact.kind);
            open_dialog(app, Dialog::QemuLaunch(QemuLaunchDialog::new(draft)));
        }
        Action::UpdateQemuLaunchDraft(draft) => {
            if let Some(Dialog::QemuLaunch(dialog)) = app.active_dialog_mut() {
                dialog.draft = draft;
                dialog.validation_error = None;
            } else {
                app.notification = Some("No runqemu launch draft is active.".into());
            }
        }
        Action::SelectQemuLaunchField { delta } => {
            if let Some(Dialog::QemuLaunch(dialog)) = app.active_dialog_mut()
                && !dialog.editing
            {
                dialog.selected_field = dialog.selected_field.shifted(delta);
            }
        }
        Action::ActivateQemuLaunchField => {
            if let Some(Dialog::QemuLaunch(dialog)) = app.active_dialog_mut() {
                if dialog.selected_field.is_read_only() {
                    app.notification = Some("Image and machine identity are read-only.".into());
                } else if dialog.selected_field.is_text() {
                    dialog.editing = true;
                    dialog.validation_error = None;
                } else if dialog.cycle_choice(false) {
                    dialog.validation_error = None;
                }
            }
        }
        Action::CycleQemuLaunchChoice { backwards } => {
            if let Some(Dialog::QemuLaunch(dialog)) = app.active_dialog_mut()
                && !dialog.editing
                && dialog.cycle_choice(backwards)
            {
                dialog.validation_error = None;
            }
        }
        Action::AppendQemuLaunchField(character) => {
            if let Some(Dialog::QemuLaunch(dialog)) = app.active_dialog_mut()
                && dialog.editing
                && !character.is_control()
                && let Some((input, maximum)) = dialog.selected_text_mut()
                && input.len() + character.len_utf8() <= maximum
            {
                input.push(character);
                dialog.validation_error = None;
            }
        }
        Action::BackspaceQemuLaunchField => {
            if let Some(Dialog::QemuLaunch(dialog)) = app.active_dialog_mut()
                && dialog.editing
                && let Some((input, _)) = dialog.selected_text_mut()
            {
                input.pop();
                dialog.validation_error = None;
            }
        }
        Action::FinishQemuLaunchFieldEdit => {
            if let Some(Dialog::QemuLaunch(dialog)) = app.active_dialog_mut() {
                dialog.editing = false;
            }
        }
        Action::PreviewQemuLaunch => {
            let Some(Dialog::QemuLaunch(dialog)) = app.active_dialog().cloned() else {
                app.notification = Some("No runqemu launch draft is active.".into());
                return None;
            };
            match dialog.draft.preview(&app.qemu_capability) {
                Ok(preview) => replace_dialog(app, Dialog::QemuLaunchConfirmation(preview)),
                Err(message) => {
                    if let Some(Dialog::QemuLaunch(dialog)) = app.active_dialog_mut() {
                        dialog.validation_error = Some(message.into());
                    }
                    app.notification = Some(message.into());
                }
            }
        }
        Action::CancelQemuLaunch => {
            if matches!(app.active_dialog(), Some(Dialog::QemuLaunch(_))) {
                close_dialog(app);
            }
        }
        Action::CancelQemuLaunchPreview => {
            if matches!(app.active_dialog(), Some(Dialog::QemuLaunchConfirmation(_))) {
                close_dialog(app);
            }
        }
        Action::ConfirmQemuLaunchInTerminal => {
            let Some(Dialog::QemuLaunchConfirmation(preview)) = app.active_dialog().cloned() else {
                app.notification = Some("No runqemu launch is awaiting confirmation.".into());
                return None;
            };
            let Some(cwd) = app.workspace.build_dir.clone() else {
                app.notification = Some("The daemon build directory is unavailable.".into());
                return None;
            };
            if !app
                .selected_image_artifact()
                .is_some_and(|artifact| artifact.identity == preview.request.image)
            {
                app.notification =
                    Some("The selected image changed; reopen the runqemu preview.".into());
                return None;
            }
            let request = &preview.request;
            let draft = QemuLaunchDraft {
                machine: request.machine.clone(),
                image: request.image.clone(),
                artifact_kind: request.artifact_kind,
                kernel: request
                    .kernel
                    .as_ref()
                    .map_or_else(String::new, |path| path.to_string_lossy().into_owned()),
                rootfs: request
                    .rootfs
                    .as_ref()
                    .map_or_else(String::new, |path| path.to_string_lossy().into_owned()),
                networking: request.networking,
                display: request.display,
                serial: request.serial,
                memory_mib: request.memory_mib.to_string(),
                extra_arguments: request.extra_arguments.join(" "),
            };
            let Ok(current) = draft.preview(&app.qemu_capability) else {
                app.notification =
                    Some("The runqemu capability changed; review the launch again.".into());
                return None;
            };
            if current != preview {
                app.notification =
                    Some("The runqemu command changed; review the launch again.".into());
                return None;
            }
            let program = preview.argv.first().cloned()?;
            let Some(arguments) = preview
                .argv
                .iter()
                .skip(1)
                .map(|value| value.to_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
            else {
                app.notification = Some("runqemu terminal arguments must be UTF-8.".into());
                return None;
            };
            close_dialog(app);
            app.screen = Screen::TerminalSessions;
            app.focus = FocusTarget::Workspace;
            app.focus_return = None;
            app.pty_selection = app.daemon.pty_sessions.len();
            app.notification = Some("QEMU console requested; press o to take writer control. Ctrl+B K terminates the session.".into());
            return Some(Effect::Terminal(TerminalEffect::Create {
                name: "QEMU console".into(),
                kind: TerminalCreationKind::QemuConsole,
                cwd,
                program,
                arguments,
            }));
        }
        Action::ConfirmQemuLaunch => {
            let Some(Dialog::QemuLaunchConfirmation(preview)) = app.active_dialog().cloned() else {
                app.notification = Some("No runqemu launch is awaiting confirmation.".into());
                return None;
            };
            if app.active_qemu_session().is_some() {
                app.notification = Some("A managed runqemu session is already active.".into());
                return None;
            }
            if preview.request.validate().is_err()
                || app
                    .qemu_capability
                    .executable_for(&preview.request.image)
                    .is_err()
            {
                app.notification =
                    Some("The runqemu launch preview is no longer valid; review it again.".into());
                return None;
            }
            while app.qemu_sessions.len() >= MAX_QEMU_SESSIONS {
                let Some(index) = app.qemu_sessions.iter().position(|session| {
                    app.background_jobs
                        .get(session.background_job_id)
                        .is_none_or(|job| job.status.is_terminal())
                }) else {
                    app.notification = Some("The runqemu session history is full.".into());
                    return None;
                };
                app.qemu_sessions.remove(index);
            }
            let id = next_qemu_session_id(app);
            let background_job_id = qemu_background_job_id(id);
            let request = preview.request;
            app.background_jobs.queue(BackgroundJobSpec {
                id: background_job_id,
                kind: BackgroundJobKind::Qemu,
                title: format!("runqemu {}", request.image.image),
                context: BackgroundJobContext {
                    workspace: Some(Screen::Images),
                    target: Some(request.image.image.clone()),
                    image: Some(request.image.image.clone()),
                    path: Some(request.image.path.clone()),
                    ..BackgroundJobContext::default()
                },
                cancellation_supported: true,
                queued_at: SystemTime::now(),
            });
            if app.background_jobs.get(background_job_id).is_none() {
                app.notification = Some("The runqemu session could not be queued.".into());
                return None;
            }
            app.qemu_sessions.push_back(QemuSession {
                id,
                background_job_id,
                request: request.clone(),
                exit_code: None,
                error_detail: None,
            });
            close_dialog(app);
            return Some(Effect::StartQemuSession { id, request });
        }
        Action::QemuSessionStarting { id, started_at } => {
            let Some(job_id) = qemu_job_id(app, id) else {
                note_stale_qemu_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Queued], |job| {
                    job.status = BackgroundJobStatus::Starting;
                    job.started_at = Some(started_at);
                });
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
