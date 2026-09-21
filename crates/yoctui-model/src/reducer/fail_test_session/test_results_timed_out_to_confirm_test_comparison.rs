use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
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
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
