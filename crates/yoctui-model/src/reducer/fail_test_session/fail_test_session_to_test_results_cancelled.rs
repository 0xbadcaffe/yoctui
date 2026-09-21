use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::FailTestSession {
            id,
            message,
            exit_code,
            finished_at,
        } => {
            let Some(Some(job_id)) = mutate_test_session(app, id, |session| {
                session.exit_code = exit_code;
                session.error_detail = Some(message.clone());
                session.outcome = Some(TestSessionOutcome::Failed);
            }) else {
                note_stale_test_event(app);
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
                        summary: "Testing operation failed".into(),
                        detail: Some(message),
                    });
                },
            );
        }
        Action::TimeoutTestSession {
            id,
            forced,
            exit_code,
            finished_at,
        } => {
            let detail = if forced {
                "Testing operation timed out and required forced termination"
            } else {
                "Testing operation timed out after graceful termination"
            };
            let Some(Some(job_id)) = mutate_test_session(app, id, |session| {
                session.exit_code = exit_code;
                session.error_detail = Some(detail.into());
                session.outcome = Some(TestSessionOutcome::TimedOut);
            }) else {
                note_stale_test_event(app);
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
                        summary: "Testing operation timed out".into(),
                        detail: Some(detail.into()),
                    });
                },
            );
        }
        Action::LoseTestSession {
            id,
            message,
            finished_at,
        } => {
            let Some(Some(job_id)) = mutate_test_session(app, id, |session| {
                session.error_detail = Some(message.clone());
                session.outcome = Some(TestSessionOutcome::Lost);
            }) else {
                note_stale_test_event(app);
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
                        summary: "Testing operation lost".into(),
                        detail: Some(message),
                    });
                },
            );
        }
        Action::BeginActiveTestSessionCancellation => {
            if let Some(id) = app.active_test_session().map(|session| session.id) {
                open_dialog(app, Dialog::TestCancellationConfirmation(id));
            } else {
                app.notification = Some("No managed Testing operation is active.".into());
            }
        }
        Action::ConfirmTestSessionCancellation => {
            let Some(Dialog::TestCancellationConfirmation(id)) = app.active_dialog().cloned()
            else {
                return None;
            };
            let Some(job_id) = test_job_id(app, id) else {
                note_stale_test_event(app);
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
                return Some(Effect::CancelTestSession(id));
            }
        }
        Action::CancelTestSessionCancellation => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::TestCancellationConfirmation(_))
            ) {
                close_dialog(app);
            }
        }
        Action::RejectTestSessionCancellation { id, message } => {
            let Some(job_id) = test_job_id(app, id) else {
                note_stale_test_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Cancelling], |job| {
                    job.status = BackgroundJobStatus::Running;
                    job.error = Some(BackgroundJobError {
                        summary: "Testing cancellation was rejected".into(),
                        detail: Some(message.clone()),
                    });
                });
            app.notification = Some(message);
        }
        Action::CancelTestSession {
            id,
            exit_code,
            finished_at,
        } => {
            let Some(Some(job_id)) = mutate_test_session(app, id, |session| {
                session.exit_code = exit_code;
                session.outcome = Some(TestSessionOutcome::Cancelled);
            }) else {
                note_stale_test_event(app);
                return None;
            };
            app.background_jobs
                .update_if(job_id, &[BackgroundJobStatus::Cancelling], |job| {
                    job.status = BackgroundJobStatus::Cancelled;
                    job.finished_at = Some(finished_at);
                    job.result = Some(BackgroundJobResult {
                        summary: "Testing operation cancelled".into(),
                        artifacts: Vec::new(),
                    });
                });
        }
        Action::InspectResultToolCapability => {
            app.result_tool_capability = ResultToolCapability::NotInspected;
            return Some(Effect::InspectResultToolCapability);
        }
        Action::ResultToolCapabilityLoaded(capability) => {
            app.result_tool_capability = capability;
        }
        Action::CycleTestView => {
            app.test_view = app.test_view.next();
        }
        Action::SelectTestView(view) => {
            app.test_view = view;
        }
        Action::BeginTestResultImport => {
            let mut editor = PopupEditor::new(popup_toml_document("root", "", None));
            let _ = editor.select_toml_value("root");
            open_dialog(
                app,
                Dialog::TestResultImportTomlEditor {
                    editor,
                    validation_error: None,
                },
            );
        }
        Action::ToggleTestResultImportTomlEditor => {
            if let Some(Dialog::TestResultImportTomlEditor { editor, .. }) = app.active_dialog_mut()
            {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendTestResultImportTomlEditor(character) => {
            if let Some(Dialog::TestResultImportTomlEditor {
                editor,
                validation_error,
            }) = app.active_dialog_mut()
                && editor.editing
                && !character.is_control()
                && editor.text.len() + character.len_utf8() <= MAX_TEST_TEXT_BYTES
            {
                editor.insert(&character.to_string());
                *validation_error = None;
            }
        }
        Action::BackspaceTestResultImportTomlEditor => {
            if let Some(Dialog::TestResultImportTomlEditor {
                editor,
                validation_error,
            }) = app.active_dialog_mut()
                && editor.editing
            {
                editor.backspace();
                *validation_error = None;
            }
        }
        Action::AppendTestResultImport(character) => {
            if let Some(Dialog::TestResultImport(dialog)) = app.active_dialog_mut() {
                dialog.append(character);
            }
        }
        Action::BackspaceTestResultImport => {
            if let Some(Dialog::TestResultImport(dialog)) = app.active_dialog_mut() {
                dialog.backspace();
            }
        }
        Action::ConfirmTestResultImport => {
            if let Some(Dialog::TestResultImportTomlEditor { editor, .. }) =
                app.active_dialog().cloned()
            {
                let root = popup_toml_value(&editor.text, "root")
                    .map(PathBuf::from)
                    .and_then(|root| {
                        absolute_normal_path(&root).then_some(root).ok_or_else(|| {
                            "result import path must be normalized and absolute".to_owned()
                        })
                    });
                match root {
                    Ok(root) => {
                        close_dialog(app);
                        return begin_test_result_import(app, vec![root]);
                    }
                    Err(message) => {
                        if let Some(Dialog::TestResultImportTomlEditor {
                            validation_error, ..
                        }) = app.active_dialog_mut()
                        {
                            *validation_error = Some(message);
                        }
                    }
                }
                return None;
            }
            let Some(Dialog::TestResultImport(dialog)) = app.active_dialog().cloned() else {
                return None;
            };
            match dialog.root() {
                Ok(root) => {
                    close_dialog(app);
                    return begin_test_result_import(app, vec![root]);
                }
                Err(message) => {
                    if let Some(Dialog::TestResultImport(dialog)) = app.active_dialog_mut() {
                        dialog.validation_error = Some(message.into());
                    }
                }
            }
        }
        Action::CancelTestResultImport => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::TestResultImport(_) | Dialog::TestResultImportTomlEditor { .. })
            ) {
                close_dialog(app);
            }
        }
        Action::RefreshTestResults => {
            let Some(roots) = app
                .test_results
                .request()
                .map(|request| request.roots.clone())
            else {
                app.notification = Some("No validated test-result roots are retained.".into());
                return None;
            };
            return begin_test_result_import(app, roots);
        }
        Action::TestResultsLoaded {
            request,
            records,
            limitations,
        } => {
            if !test_result_request_is_current(app, &request)
                || records.iter().any(|record| !record.is_valid())
            {
                note_stale_test_event(app);
                return None;
            }
            let previous = app.test_result_selection.clone();
            let (records, limitations) = normalize_test_results(records, limitations);
            app.test_results = if records.is_empty() && limitations.is_empty() {
                TestResultInventoryState::AvailableEmpty { request }
            } else if limitations.is_empty() {
                TestResultInventoryState::Available { request, records }
            } else {
                TestResultInventoryState::Partial {
                    request,
                    records,
                    limitations,
                }
            };
            set_test_result_selection_to_current_or_first(app, previous);
            if app
                .test_comparison
                .request()
                .is_some_and(|request| !test_comparison_inputs_exist(app, request))
            {
                app.test_comparison = TestComparisonState::NotSelected;
                app.test_comparison_selection = None;
            }
        }
        Action::TestResultsFailed { request, message } => {
            if !test_result_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_results = TestResultInventoryState::Failed {
                request,
                message: message.clone(),
            };
            app.notification = Some(format!("Test result import failed: {message}"));
        }
        Action::TestResultsCancelled { request } => {
            if !test_result_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_results = TestResultInventoryState::Cancelled { request };
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
