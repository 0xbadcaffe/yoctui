use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::LoseSdkSession {
            id,
            message,
            finished_at,
        } => {
            let Some(job_id) = mutate_sdk_session(app, id, |session| {
                session.error_detail = Some(message.clone())
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
                    job.status = BackgroundJobStatus::Lost;
                    job.finished_at = Some(finished_at);
                    job.error = Some(BackgroundJobError {
                        summary: "SDK operation lost".into(),
                        detail: Some(message),
                    });
                },
            );
        }
        Action::BeginActiveSdkSessionCancellation => {
            if let Some(id) = app.active_sdk_session().map(|session| session.id) {
                open_dialog(app, Dialog::SdkCancellationConfirmation(id));
            } else if matches!(app.sdk_artifacts, SdkArtifactInventoryState::Loading { .. }) {
                return Some(Effect::CancelSdkArtifactOperation);
            } else {
                app.notification = Some("No managed SDK operation is active.".into());
            }
        }
        Action::ConfirmSdkSessionCancellation => {
            let Some(Dialog::SdkCancellationConfirmation(id)) = app.active_dialog().cloned() else {
                return None;
            };
            let Some(job_id) = sdk_job_id(app, id) else {
                note_stale_sdk_event(app);
                close_dialog(app);
                return None;
            };
            let before = app.background_jobs.get(job_id).map(|job| job.status);
            app.background_jobs.update_if(
                job_id,
                &[
                    BackgroundJobStatus::Queued,
                    BackgroundJobStatus::Starting,
                    BackgroundJobStatus::Running,
                ],
                |job| job.status = BackgroundJobStatus::Cancelling,
            );
            close_dialog(app);
            if before != app.background_jobs.get(job_id).map(|job| job.status) {
                return Some(Effect::CancelSdkSession(id));
            }
        }
        Action::CancelSdkSessionCancellation => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::SdkCancellationConfirmation(_))
            ) {
                close_dialog(app);
            }
        }
        Action::RejectSdkSessionCancellation { id, message } => {
            let Some(job_id) = sdk_job_id(app, id) else {
                note_stale_sdk_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Cancelling], |job| {
                    job.status = BackgroundJobStatus::Running;
                    job.error = Some(BackgroundJobError {
                        summary: "SDK cancellation was rejected".into(),
                        detail: Some(message.clone()),
                    });
                });
            app.notification = Some(message);
        }
        Action::CancelSdkSession {
            id,
            exit_code,
            finished_at,
        } => {
            let Some(job_id) = mutate_sdk_session(app, id, |session| session.exit_code = exit_code)
            else {
                note_stale_sdk_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Cancelling], |job| {
                    job.status = BackgroundJobStatus::Cancelled;
                    job.finished_at = Some(finished_at);
                    job.result = Some(BackgroundJobResult {
                        summary: "SDK operation cancelled".into(),
                        artifacts: Vec::new(),
                    });
                });
        }
        Action::InspectTestCapability => {
            app.test_capability = TestCapability::default();
            return Some(Effect::InspectTestCapability);
        }
        Action::TestCapabilityLoaded(capability) => {
            app.test_capability = capability;
            if app.screen == Screen::Testing
                && matches!(
                    app.result_tool_capability,
                    ResultToolCapability::NotInspected
                )
            {
                return Some(Effect::InspectResultToolCapability);
            }
        }
        Action::SelectTestFamily { delta } => {
            app.test_family_selection = app.test_family_selection.shifted(delta);
        }
        Action::BeginSelectedTestLaunch => {
            let draft = test_launch_draft(app, app.test_family_selection);
            let mut editor = PopupEditor::new(format!(
                "# family, machine, distro, and image are authoritative\nfamily = \"{}\"\nmachine = \"{}\"\ndistro = \"{}\"\nimage = \"{}\"\nscope = \"all\"\nselector = \"\"\nparallelism = 1\nverbose = false\nskip_network = false\n",
                draft.family.label(),
                draft.machine,
                draft.distro,
                draft.image
            ));
            let _ = editor.select_toml_value("scope");
            open_dialog(
                app,
                Dialog::TestLaunchTomlEditor {
                    family: draft.family,
                    editor,
                    validation_error: None,
                },
            );
        }
        Action::ToggleTestLaunchTomlEditor => {
            if let Some(Dialog::TestLaunchTomlEditor { editor, .. }) = app.active_dialog_mut() {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendTestLaunchTomlEditor(character) => {
            if let Some(Dialog::TestLaunchTomlEditor { editor, .. }) = app.active_dialog_mut()
                && editor.editing
                && !character.is_control()
                && editor.text.len() + character.len_utf8() <= 16_384
            {
                editor.insert(&character.to_string());
            }
        }
        Action::BackspaceTestLaunchTomlEditor => {
            if let Some(Dialog::TestLaunchTomlEditor { editor, .. }) = app.active_dialog_mut()
                && editor.editing
            {
                editor.backspace();
            }
        }
        Action::UpdateTestLaunchDraft(draft) => {
            if matches!(app.active_dialog(), Some(Dialog::TestLaunch(_))) {
                replace_dialog(app, Dialog::TestLaunch(TestLaunchDialog::new(draft)));
            }
        }
        Action::SelectTestLaunchField { delta } => {
            if let Some(Dialog::TestLaunch(dialog)) = app.active_dialog_mut()
                && !dialog.editing
            {
                dialog.select(delta);
            }
        }
        Action::ActivateTestLaunchField => {
            if let Some(Dialog::TestLaunch(dialog)) = app.active_dialog_mut() {
                dialog.activate();
            }
        }
        Action::AppendTestLaunchField(character) => {
            if let Some(Dialog::TestLaunch(dialog)) = app.active_dialog_mut() {
                dialog.append(character);
            }
        }
        Action::BackspaceTestLaunchField => {
            if let Some(Dialog::TestLaunch(dialog)) = app.active_dialog_mut() {
                dialog.backspace();
            }
        }
        Action::FinishTestLaunchFieldEdit => {
            if let Some(Dialog::TestLaunch(dialog)) = app.active_dialog_mut() {
                dialog.finish_edit();
            }
        }
        Action::PreviewTestLaunch => {
            if let Some(Dialog::TestLaunchTomlEditor { editor, .. }) = app.active_dialog().cloned()
            {
                let preview = (|| {
                    let fields = popup_toml_fields(&editor.text)?;
                    let get = |key: &str| {
                        fields
                            .get(key)
                            .cloned()
                            .ok_or_else(|| format!("Missing `{key}`."))
                    };
                    let family = match get("family")?.as_str() {
                        "OE selftest" => TestFamily::OeSelftest,
                        "BitBake selftest" => TestFamily::BitbakeSelftest,
                        "Image runtime" => TestFamily::TestImage,
                        "Standard SDK" => TestFamily::TestSdk,
                        "Extensible SDK" => TestFamily::TestSdkExt,
                        "Package tests" => TestFamily::Ptest,
                        _ => return Err("Unknown test family.".to_owned()),
                    };
                    let scope = match get("scope")?.as_str() {
                        "all" => TestSelectorScope::All,
                        "selected" => TestSelectorScope::Selected,
                        _ => return Err("`scope` must be all or selected.".to_owned()),
                    };
                    let parallelism = get("parallelism")?
                        .parse()
                        .map_err(|_| "`parallelism` must be a number.".to_owned())?;
                    let boolean = |key: &str| match get(key)?.as_str() {
                        "true" => Ok(true),
                        "false" => Ok(false),
                        _ => Err(format!("`{key}` must be true or false.")),
                    };
                    let draft = TestLaunchDraft {
                        family,
                        machine: get("machine")?,
                        distro: get("distro")?,
                        image: get("image")?,
                        scope,
                        selector: get("selector")?,
                        parallelism,
                        verbose: boolean("verbose")?,
                        skip_network: boolean("skip_network")?,
                    };
                    let authoritative = test_launch_draft(app, app.test_family_selection);
                    if draft.family != authoritative.family
                        || draft.machine != authoritative.machine
                        || draft.distro != authoritative.distro
                        || draft.image != authoritative.image
                    {
                        return Err(
                            "`family`, `machine`, `distro`, and `image` must match the current Testing context."
                                .to_owned(),
                        );
                    }
                    draft.preview(&app.test_capability).map_err(str::to_owned)
                })();
                match preview {
                    Ok(preview) => replace_dialog(app, Dialog::TestLaunchConfirmation(preview)),
                    Err(message) => {
                        if let Some(Dialog::TestLaunchTomlEditor {
                            validation_error, ..
                        }) = app.active_dialog_mut()
                        {
                            *validation_error = Some(message);
                        }
                    }
                }
                return None;
            }
            let Some(Dialog::TestLaunch(dialog)) = app.active_dialog().cloned() else {
                return None;
            };
            match dialog.draft.preview(&app.test_capability) {
                Ok(preview) => {
                    replace_dialog(app, Dialog::TestLaunchConfirmation(preview));
                }
                Err(message) => {
                    if let Some(Dialog::TestLaunch(dialog)) = app.active_dialog_mut() {
                        dialog.validation_error = Some(message.into());
                    }
                }
            }
        }
        Action::CancelTestLaunch => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::TestLaunch(_) | Dialog::TestLaunchTomlEditor { .. })
            ) {
                close_dialog(app);
            }
        }
        Action::CancelTestLaunchPreview => {
            if matches!(app.active_dialog(), Some(Dialog::TestLaunchConfirmation(_))) {
                close_dialog(app);
            }
        }
        Action::ConfirmTestLaunch => {
            let Some(Dialog::TestLaunchConfirmation(preview)) = app.active_dialog().cloned() else {
                return None;
            };
            if !test_preview_is_current(app, &preview) {
                app.notification = Some("The Testing launch preview is stale.".into());
                return None;
            }
            close_dialog(app);
            return queue_test_session(app, preview.operation());
        }
        Action::AttachTestBuildSession {
            id,
            background_job_id,
        } => {
            let valid = app.test_session(id).is_some_and(|session| {
                session.background_job_id.is_none()
                    && matches!(session.operation, TestOperation::Build { .. })
            }) && app
                .background_jobs
                .get(background_job_id)
                .is_some_and(|job| job.kind == BackgroundJobKind::Test);
            if !valid {
                note_stale_test_event(app);
                return None;
            }
            let _ = mutate_test_session(app, id, |session| {
                session.background_job_id = Some(background_job_id)
            });
        }
        Action::TestSessionStarting { id, started_at } => {
            let Some(job_id) = test_job_id(app, id) else {
                note_stale_test_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Queued], |job| {
                    job.status = BackgroundJobStatus::Starting;
                    job.started_at = Some(started_at);
                });
        }
        Action::TestSessionRunning { id } => {
            let Some(job_id) = test_job_id(app, id) else {
                note_stale_test_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Starting], |job| {
                    job.status = BackgroundJobStatus::Running;
                });
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
