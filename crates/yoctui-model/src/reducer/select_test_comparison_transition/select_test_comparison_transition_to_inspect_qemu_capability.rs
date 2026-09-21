use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::SelectTestComparisonTransition { delta } => {
            let identities = app
                .test_comparison_transitions()
                .iter()
                .map(|transition| transition.identity.clone())
                .collect::<Vec<_>>();
            if identities.is_empty() {
                app.test_comparison_selection = None;
                return None;
            }
            let current = app
                .test_comparison_selection
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
            app.test_comparison_selection = identities.get(next).cloned();
        }
        Action::OpenSelectedTestTransitionLog => {
            let Some(path) = app.selected_test_transition().and_then(|transition| {
                transition
                    .candidate_log
                    .clone()
                    .or_else(|| transition.baseline_log.clone())
            }) else {
                app.notification =
                    Some("The selected comparison transition has no exact log path.".into());
                return None;
            };
            return Some(Effect::OpenInEditor(path));
        }
        Action::BeginTestJunitExport => {
            let Some(identity) = app
                .selected_test_result()
                .map(|record| record.identity.clone())
            else {
                app.notification = Some("No exact test result is selected for export.".into());
                return None;
            };
            if app.result_tool_capability.executable().is_err() {
                app.notification = Some("resulttool is unavailable for JUnit export.".into());
                return None;
            }
            let mut editor = PopupEditor::new(popup_toml_document("destination", "", None));
            let _ = editor.select_toml_value("destination");
            open_dialog(
                app,
                Dialog::TestJunitTomlEditor {
                    result: identity,
                    editor,
                    validation_error: None,
                },
            );
        }
        Action::ToggleTestJunitTomlEditor => {
            if let Some(Dialog::TestJunitTomlEditor { editor, .. }) = app.active_dialog_mut() {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendTestJunitTomlEditor(character) => {
            if let Some(Dialog::TestJunitTomlEditor {
                editor,
                validation_error,
                ..
            }) = app.active_dialog_mut()
                && editor.editing
                && !character.is_control()
                && editor.text.len() < MAX_TEST_TEXT_BYTES
            {
                editor.insert(&character.to_string());
                *validation_error = None;
            }
        }
        Action::BackspaceTestJunitTomlEditor => {
            if let Some(Dialog::TestJunitTomlEditor {
                editor,
                validation_error,
                ..
            }) = app.active_dialog_mut()
                && editor.editing
            {
                editor.backspace();
                *validation_error = None;
            }
        }
        Action::MoveTestJunitTomlEditorLeft => {
            if let Some(Dialog::TestJunitTomlEditor { editor, .. }) = app.active_dialog_mut() {
                editor.left();
            }
        }
        Action::MoveTestJunitTomlEditorRight => {
            if let Some(Dialog::TestJunitTomlEditor { editor, .. }) = app.active_dialog_mut() {
                editor.right();
            }
        }
        Action::MoveTestJunitTomlEditorUp => {
            if let Some(Dialog::TestJunitTomlEditor { editor, .. }) = app.active_dialog_mut() {
                editor.up();
            }
        }
        Action::MoveTestJunitTomlEditorDown => {
            if let Some(Dialog::TestJunitTomlEditor { editor, .. }) = app.active_dialog_mut() {
                editor.down();
            }
        }
        Action::MoveTestJunitTomlEditorHome => {
            if let Some(Dialog::TestJunitTomlEditor { editor, .. }) = app.active_dialog_mut() {
                editor.home();
            }
        }
        Action::MoveTestJunitTomlEditorEnd => {
            if let Some(Dialog::TestJunitTomlEditor { editor, .. }) = app.active_dialog_mut() {
                editor.end();
            }
        }
        Action::SelectTestJunitDestination => {
            if let Some(Dialog::TestJunitTomlEditor {
                editor,
                validation_error,
                ..
            }) = app.active_dialog_mut()
            {
                *validation_error = editor.select_toml_value("destination").err();
                editor.editing = validation_error.is_none();
            }
        }
        Action::CopyTestJunitTomlEditor => {
            if let Some(Dialog::TestJunitTomlEditor { editor, .. }) = app.active_dialog_mut() {
                return Some(Effect::CopyToClipboard(editor.copy_selection_or_line()));
            }
        }
        Action::PasteTestJunitTomlEditor => {
            if let Some(Dialog::TestJunitTomlEditor {
                editor,
                validation_error,
                ..
            }) = app.active_dialog_mut()
                && editor.editing
            {
                editor.paste();
                *validation_error = None;
            }
        }
        Action::AppendTestJunitDestination(character) => {
            if let Some(Dialog::TestJunitExport(dialog)) = app.active_dialog_mut() {
                dialog.append(character);
            }
        }
        Action::BackspaceTestJunitDestination => {
            if let Some(Dialog::TestJunitExport(dialog)) = app.active_dialog_mut() {
                dialog.backspace();
            }
        }
        Action::PreviewTestJunitExport => {
            if let Some(Dialog::TestJunitTomlEditor { result, editor, .. }) =
                app.active_dialog().cloned()
            {
                let destination = popup_toml_value(&editor.text, "destination")
                    .map(PathBuf::from)
                    .and_then(|path| {
                        (absolute_normal_path(&path)
                            && path.extension().and_then(|value| value.to_str()) == Some("xml"))
                        .then_some(path)
                        .ok_or_else(|| {
                            "JUnit destination must be a normalized absolute .xml path".to_owned()
                        })
                    });
                match destination {
                    Ok(destination) => {
                        app.test_junit_export = TestJunitExportState::Inspecting {
                            result: result.clone(),
                            destination: destination.clone(),
                        };
                        return Some(Effect::InspectTestJunitDestination {
                            result,
                            destination,
                        });
                    }
                    Err(message) => {
                        if let Some(Dialog::TestJunitTomlEditor {
                            validation_error, ..
                        }) = app.active_dialog_mut()
                        {
                            *validation_error = Some(message);
                        }
                    }
                }
                return None;
            }
            let Some(Dialog::TestJunitExport(dialog)) = app.active_dialog().cloned() else {
                return None;
            };
            match dialog.lexical_destination() {
                Ok(destination) => {
                    app.test_junit_export = TestJunitExportState::Inspecting {
                        result: dialog.result.clone(),
                        destination: destination.clone(),
                    };
                    return Some(Effect::InspectTestJunitDestination {
                        result: dialog.result,
                        destination,
                    });
                }
                Err(message) => {
                    if let Some(Dialog::TestJunitExport(dialog)) = app.active_dialog_mut() {
                        dialog.validation_error = Some(message.into());
                    }
                }
            }
        }
        Action::CancelTestJunitExport => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::TestJunitExport(_) | Dialog::TestJunitTomlEditor { .. })
            ) {
                close_dialog(app);
                app.test_junit_export = TestJunitExportState::NotStarted;
            }
        }
        Action::TestJunitDestinationInspected { result, inspection } => {
            let current = matches!(
                &app.test_junit_export,
                TestJunitExportState::Inspecting {
                    result: current_result,
                    destination,
                } if current_result == &result && destination == &inspection.requested
            ) && app
                .selected_test_result()
                .is_some_and(|record| record.identity == result)
                && matches!(app.active_dialog(),
                    Some(Dialog::TestJunitExport(dialog)) if dialog.result == result)
                || matches!(app.active_dialog(),
                        Some(Dialog::TestJunitTomlEditor { result: dialog_result, .. }) if *dialog_result == result);
            if !current {
                note_stale_test_event(app);
                return None;
            }
            app.test_junit_generation = app.test_junit_generation.wrapping_add(1).max(1);
            let preview =
                TestJunitExportRequest::new(app.test_junit_generation, result, &inspection)
                    .and_then(|request| {
                        app.result_tool_capability
                            .executable()
                            .and_then(|executable| TestJunitExportPreview::new(executable, request))
                    });
            match preview {
                Ok(preview) => {
                    app.test_junit_export = TestJunitExportState::Ready(preview.clone());
                    replace_dialog(app, Dialog::TestJunitExportConfirmation(preview));
                }
                Err(message) => {
                    app.test_junit_export = TestJunitExportState::NotStarted;
                    match app.active_dialog_mut() {
                        Some(Dialog::TestJunitExport(dialog)) => {
                            dialog.validation_error = Some(message.into())
                        }
                        Some(Dialog::TestJunitTomlEditor {
                            validation_error, ..
                        }) => *validation_error = Some(message.into()),
                        _ => {}
                    }
                }
            }
        }
        Action::CancelTestJunitExportPreview => {
            if matches!(
                app.active_dialog(),
                Some(Dialog::TestJunitExportConfirmation(_))
            ) {
                close_dialog(app);
                app.test_junit_export = TestJunitExportState::NotStarted;
            }
        }
        Action::ConfirmTestJunitExport => {
            let Some(Dialog::TestJunitExportConfirmation(preview)) = app.active_dialog().cloned()
            else {
                return None;
            };
            let valid = matches!(
                &app.test_junit_export,
                TestJunitExportState::Ready(current) if current == &preview
            ) && app
                .test_results
                .records()
                .iter()
                .any(|record| record.identity == preview.request.result)
                && app
                    .result_tool_capability
                    .executable()
                    .is_ok_and(|executable| preview.argv.first() == Some(&executable));
            if !valid {
                app.notification = Some("The JUnit export preview is stale.".into());
                return None;
            }
            close_dialog(app);
            let request = preview.request;
            app.test_junit_export = TestJunitExportState::Running(request.clone());
            return Some(Effect::ExportTestJunit(request));
        }
        Action::TestJunitExportSucceeded { request } => {
            if !test_junit_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_junit_export = TestJunitExportState::Succeeded(request);
        }
        Action::TestJunitExportFailed { request, message } => {
            if !test_junit_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_junit_export = TestJunitExportState::Failed {
                request,
                message: message.clone(),
            };
            app.notification = Some(format!("JUnit export failed: {message}"));
        }
        Action::TestJunitExportCancelled { request } => {
            if !test_junit_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_junit_export = TestJunitExportState::Cancelled(request);
        }
        Action::TestJunitExportTimedOut { request } => {
            if !test_junit_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_junit_export = TestJunitExportState::TimedOut(request);
        }
        Action::TestJunitExportLost { request, message } => {
            if !test_junit_request_is_current(app, &request) {
                note_stale_test_event(app);
                return None;
            }
            app.test_junit_export = TestJunitExportState::Lost { request, message };
        }
        Action::InspectQemuCapability => {
            app.qemu_capability = QemuCapability::NotInspected;
            return Some(Effect::InspectQemuCapability);
        }
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
