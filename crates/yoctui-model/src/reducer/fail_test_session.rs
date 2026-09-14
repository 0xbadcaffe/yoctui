//! State transitions beginning with FailTestSession.
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
        Action::TestResultsTimedOut { request } => {
            if !test_result_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_results = TestResultInventoryState::TimedOut { request };
        }
        Action::TestResultsLost { request, message } => {
            if !test_result_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_results = TestResultInventoryState::Lost {
                request,
                message: message.clone(),
            };
            app.notification = Some(format!("Test result worker was lost: {message}"));
        }
        Action::SelectTestResult { delta } => {
            let visible = app
                .filtered_test_results()
                .into_iter()
                .map(|record| record.identity.clone())
                .collect::<Vec<_>>();
            if visible.is_empty() {
                app.test_result_selection = None;
                return None;
            }
            let current = app
                .test_result_selection
                .as_ref()
                .and_then(|identity| visible.iter().position(|candidate| candidate == identity))
                .unwrap_or_default();
            let next = if delta.is_negative() {
                current.saturating_sub(delta.unsigned_abs())
            } else {
                current
                    .saturating_add(delta as usize)
                    .min(visible.len() - 1)
            };
            app.test_result_selection = visible.get(next).cloned();
            app.test_result_drilled = false;
            app.test_case_selection = None;
        }
        Action::BeginTestResultSearch => app.test_result_searching = true,
        Action::AppendTestResultQuery(character) => {
            if app.test_result_searching
                && !character.is_control()
                && app.test_result_query.len() + character.len_utf8() <= MAX_TEST_TEXT_BYTES
            {
                app.test_result_query.push(character);
                let previous = app.test_result_selection.clone();
                set_test_result_selection_to_current_or_first(app, previous);
            }
        }
        Action::BackspaceTestResultQuery => {
            if app.test_result_searching {
                app.test_result_query.pop();
                let previous = app.test_result_selection.clone();
                set_test_result_selection_to_current_or_first(app, previous);
            }
        }
        Action::ClearTestResultQuery => {
            app.test_result_query.clear();
            let previous = app.test_result_selection.clone();
            set_test_result_selection_to_current_or_first(app, previous);
        }
        Action::FinishTestResultSearch => app.test_result_searching = false,
        Action::OpenSelectedTestResult => {
            let Some(record) = app.selected_test_result() else {
                app.notification = Some("No exact test result is selected.".into());
                return None;
            };
            return Some(Effect::OpenInEditor(record.identity.path.clone()));
        }
        Action::DrillIntoSelectedTestResult => {
            let first = app.selected_test_result().and_then(|record| {
                record
                    .suites
                    .iter()
                    .flat_map(|suite| &suite.cases)
                    .next()
                    .map(|case| case.identity.clone())
            });
            if first.is_some() {
                app.test_result_drilled = true;
                app.test_case_selection = first;
            } else {
                app.notification = Some("The selected test result contains no cases.".into());
            }
        }
        Action::LeaveTestResultCases => {
            app.test_result_drilled = false;
            app.test_case_selection = None;
        }
        Action::SelectTestCase { delta } => {
            let identities = app.selected_test_result().map_or_else(Vec::new, |record| {
                record
                    .suites
                    .iter()
                    .flat_map(|suite| &suite.cases)
                    .map(|case| case.identity.clone())
                    .collect::<Vec<_>>()
            });
            if identities.is_empty() {
                app.test_case_selection = None;
                return None;
            }
            let current = app
                .test_case_selection
                .as_ref()
                .and_then(|identity| {
                    identities
                        .iter()
                        .position(|candidate| candidate == identity)
                })
                .unwrap_or_default();
            let next = if delta.is_negative() {
                current.saturating_sub(delta.unsigned_abs())
            } else {
                current
                    .saturating_add(delta as usize)
                    .min(identities.len() - 1)
            };
            app.test_case_selection = identities.get(next).cloned();
        }
        Action::OpenSelectedTestCaseLog => {
            let Some(path) = app
                .selected_test_case()
                .and_then(|case| case.log_path.clone())
            else {
                app.notification = Some("The selected test case has no exact log path.".into());
                return None;
            };
            return Some(Effect::OpenInEditor(path));
        }
        Action::BeginTestComparison => {
            let records = app.test_results.records();
            if records.len() < 2 {
                app.notification =
                    Some("At least two exact test results are required for comparison.".into());
                return None;
            }
            let picker = TestComparisonPicker::new(app.test_result_selection.clone(), records);
            let mut editor = PopupEditor::new(format!(
                "baseline = \"{}\"\ncandidate = \"{}\"\n",
                picker
                    .baseline
                    .as_ref()
                    .map_or_else(String::new, |value| value.path.display().to_string()),
                picker
                    .candidate
                    .as_ref()
                    .map_or_else(String::new, |value| value.path.display().to_string())
            ));
            let _ = editor.select_toml_value("baseline");
            open_dialog(
                app,
                Dialog::TestComparisonTomlEditor {
                    editor,
                    validation_error: None,
                },
            );
        }
        Action::ToggleTestComparisonTomlEditor => {
            if let Some(Dialog::TestComparisonTomlEditor { editor, .. }) = app.active_dialog_mut() {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendTestComparisonTomlEditor(character) => {
            if let Some(Dialog::TestComparisonTomlEditor {
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
        Action::BackspaceTestComparisonTomlEditor => {
            if let Some(Dialog::TestComparisonTomlEditor {
                editor,
                validation_error,
            }) = app.active_dialog_mut()
                && editor.editing
            {
                editor.backspace();
                *validation_error = None;
            }
        }
        Action::SelectTestComparisonChoice { delta } => {
            let records = app.test_results.records().to_vec();
            if let Some(Dialog::TestComparison(dialog)) = app.active_dialog_mut() {
                dialog.select(&records, delta);
            }
        }
        Action::CycleTestComparisonField => {
            if let Some(Dialog::TestComparison(dialog)) = app.active_dialog_mut() {
                dialog.cycle_field();
            }
        }
        Action::ActivateTestComparisonChoice => {
            if let Some(Dialog::TestComparison(dialog)) = app.active_dialog_mut() {
                dialog.activate();
            }
        }
        Action::PreviewTestComparison => {
            if let Some(Dialog::TestComparisonTomlEditor { editor, .. }) =
                app.active_dialog().cloned()
            {
                let preview = (|| {
                    let fields = popup_toml_fields(&editor.text)?;
                    let lookup = |key: &str| {
                        let path = fields.get(key).ok_or_else(|| format!("Missing `{key}`."))?;
                        app.test_results
                            .records()
                            .iter()
                            .find(|record| record.identity.path == Path::new(path))
                            .map(|record| record.identity.clone())
                            .ok_or_else(|| format!("`{key}` is not an available test result."))
                    };
                    app.test_comparison_generation =
                        app.test_comparison_generation.wrapping_add(1).max(1);
                    let request = TestComparisonRequest::new(
                        app.test_comparison_generation,
                        lookup("baseline")?,
                        lookup("candidate")?,
                    )
                    .map_err(str::to_owned)?;
                    let executable = app
                        .result_tool_capability
                        .executable()
                        .map_err(str::to_owned)?;
                    TestComparisonPreview::new(executable, request).map_err(str::to_owned)
                })();
                match preview {
                    Ok(preview) => replace_dialog(app, Dialog::TestComparisonConfirmation(preview)),
                    Err(message) => {
                        if let Some(Dialog::TestComparisonTomlEditor {
                            validation_error, ..
                        }) = app.active_dialog_mut()
                        {
                            *validation_error = Some(message);
                        }
                    }
                }
                return None;
            }
            let Some(Dialog::TestComparison(dialog)) = app.active_dialog().cloned() else {
                return None;
            };
            app.test_comparison_generation = app.test_comparison_generation.wrapping_add(1).max(1);
            let preview = dialog
                .preview(app.test_comparison_generation)
                .and_then(|request| {
                    app.result_tool_capability
                        .executable()
                        .and_then(|executable| TestComparisonPreview::new(executable, request))
                });
            match preview {
                Ok(preview) => {
                    replace_dialog(app, Dialog::TestComparisonConfirmation(preview));
                }
                Err(message) => {
                    if let Some(Dialog::TestComparison(dialog)) = app.active_dialog_mut() {
                        dialog.validation_error = Some(message.into());
                    }
                }
            }
        }
        Action::CancelTestComparison => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::TestComparison(_) | Dialog::TestComparisonTomlEditor { .. })
            ) {
                close_dialog(app);
            }
        }
        Action::CancelTestComparisonPreview => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::TestComparisonConfirmation(_))
            ) {
                close_dialog(app);
            }
        }
        Action::ConfirmTestComparison => {
            let Some(Dialog::TestComparisonConfirmation(preview)) = app.active_dialog().cloned()
            else {
                return None;
            };
            let request = preview.request;
            let valid = app
                .test_results
                .records()
                .iter()
                .any(|record| record.identity == request.baseline)
                && app
                    .test_results
                    .records()
                    .iter()
                    .any(|record| record.identity == request.candidate)
                && app
                    .result_tool_capability
                    .executable()
                    .is_ok_and(|executable| preview.argv.first() == Some(&executable));
            if !valid {
                app.notification = Some("The test comparison preview is stale.".into());
                return None;
            }
            close_dialog(app);
            app.test_comparison = TestComparisonState::Loading {
                request: request.clone(),
            };
            return Some(Effect::CompareTestResults(request));
        }
        Action::TestComparisonLoaded {
            request,
            comparison,
            limitations,
        } => {
            if !test_comparison_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            let baseline = app
                .test_results
                .records()
                .iter()
                .find(|record| record.identity == request.baseline);
            let candidate = app
                .test_results
                .records()
                .iter()
                .find(|record| record.identity == request.candidate);
            let Some(expected) = baseline.zip(candidate).and_then(|(baseline, candidate)| {
                TestComparison::between(baseline, candidate).ok()
            }) else {
                note_stale_test_event(app);
                return None;
            };
            if comparison != expected {
                note_stale_test_event(app);
                app.notification =
                    Some("Testing rejected an inconsistent comparison result.".into());
                return None;
            }
            let limitations = normalize_limitations(limitations);
            app.test_comparison = if limitations.is_empty() {
                TestComparisonState::Available {
                    request,
                    comparison,
                }
            } else {
                TestComparisonState::Partial {
                    request,
                    comparison,
                    limitations,
                }
            };
            set_test_comparison_selection(app);
        }
        Action::TestComparisonFailed { request, message } => {
            if !test_comparison_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_comparison = TestComparisonState::Failed {
                request,
                message: message.clone(),
            };
            app.notification = Some(format!("Test comparison failed: {message}"));
        }
        Action::TestComparisonCancelled { request } => {
            if !test_comparison_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_comparison = TestComparisonState::Cancelled { request };
        }
        Action::TestComparisonTimedOut { request } => {
            if !test_comparison_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_comparison = TestComparisonState::TimedOut { request };
        }
        Action::TestComparisonLost { request, message } => {
            if !test_comparison_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_comparison = TestComparisonState::Lost { request, message };
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
