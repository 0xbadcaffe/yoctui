//! State transitions beginning with ConfirmKeymapCapture.
use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
        Action::ConfirmKeymapCapture => {
            let row = app
                .keymap_preferences_ui
                .selected_row(&app.keymap_preferences, &app.effective_keymap)?;
            let Some(sequence) = app
                .keymap_preferences_ui
                .capture
                .as_ref()
                .and_then(KeymapCaptureState::sequence)
            else {
                app.keymap_preferences_ui.validation_error =
                    Some("Capture at least one key before saving.".into());
                return None;
            };
            let candidate = app.keymap_preferences.with_action_sequences(
                row.action_id,
                row.scope,
                vec![sequence],
            );
            match EffectiveKeymap::from_preferences(&candidate) {
                Ok(effective) => {
                    app.keymap_preferences = candidate;
                    app.effective_keymap = effective;
                    app.keymap_preferences_ui.capture = None;
                    app.keymap_preferences_ui.validation_error = None;
                    app.settings_dirty = true;
                    return Some(Effect::PersistSettings);
                }
                Err(error) => {
                    app.keymap_preferences_ui.validation_error = Some(error.to_string());
                }
            }
        }
        Action::CancelKeymapCapture => {
            app.keymap_preferences_ui.capture = None;
            app.keymap_preferences_ui.validation_error = None;
        }
        Action::RemoveKeymapBinding => {
            let row = app
                .keymap_preferences_ui
                .selected_row(&app.keymap_preferences, &app.effective_keymap)?;
            let candidate =
                app.keymap_preferences
                    .with_action_sequences(row.action_id, row.scope, Vec::new());
            match EffectiveKeymap::from_preferences(&candidate) {
                Ok(effective) => {
                    app.keymap_preferences = candidate;
                    app.effective_keymap = effective;
                    app.keymap_preferences_ui.validation_error = None;
                    app.settings_dirty = true;
                    return Some(Effect::PersistSettings);
                }
                Err(error) => {
                    app.keymap_preferences_ui.validation_error = Some(error.to_string());
                }
            }
        }
        Action::ResetKeymapBinding => {
            let row = app
                .keymap_preferences_ui
                .selected_row(&app.keymap_preferences, &app.effective_keymap)?;
            let candidate = app
                .keymap_preferences
                .reset_action(row.action_id, row.scope);
            if let Ok(effective) = EffectiveKeymap::from_preferences(&candidate) {
                app.keymap_preferences = candidate;
                app.effective_keymap = effective;
                app.keymap_preferences_ui.validation_error = None;
                app.settings_dirty = true;
                return Some(Effect::PersistSettings);
            }
        }
        Action::ResetAllKeymapBindings => {
            app.reset_keymap();
            app.keymap_preferences_ui.open = true;
            app.settings_dirty = true;
            return Some(Effect::PersistSettings);
        }
        Action::ExportEffectiveKeymap if app.keymap_preferences_ui.open => {
            return Some(Effect::CopyToClipboard(app.effective_keymap_report()));
        }
        Action::ExportEffectiveKeymap => {}
        Action::SettingsPersisted => {
            app.preferences = app.effective_preferences();
            app.settings_dirty = false;
            app.notification = None;
        }
        Action::SettingsPersistenceFailed(message) => {
            app.settings_dirty = true;
            app.notification = Some(format!(
                "Settings changed in memory but could not be saved: {message}"
            ));
        }
        Action::SetCompatibilityFilter(filter) => {
            app.compatibility_ui
                .set_filter(filter, app.workspace_compatibility.authority());
        }
        Action::SelectCompatibilityCapability { delta } => {
            app.compatibility_ui
                .select(delta, app.workspace_compatibility.authority());
        }
        Action::BeginCompatibilitySearch => app.compatibility_ui.begin_search(),
        Action::AppendCompatibilityQuery(character) if app.compatibility_ui.searching => {
            let _ = app
                .compatibility_ui
                .append_query(character, app.workspace_compatibility.authority());
        }
        Action::AppendCompatibilityQuery(_) => {}
        Action::BackspaceCompatibilityQuery if app.compatibility_ui.searching => {
            app.compatibility_ui
                .backspace_query(app.workspace_compatibility.authority());
        }
        Action::BackspaceCompatibilityQuery => {}
        Action::ClearCompatibilityQuery => app
            .compatibility_ui
            .clear_query(app.workspace_compatibility.authority()),
        Action::FinishCompatibilitySearch => app.compatibility_ui.finish_search(),
        Action::RawMode(RawModeAction::ConfirmPreview) => {
            app.raw_request_generation = app.raw_request_generation.wrapping_add(1).max(1);
            let nanos = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let request_id = RawRequestId::new(format!(
                "raw-request:client-{nanos}-{}",
                app.raw_request_generation
            ))
            .expect("generated Raw request identity is bounded and valid");
            let authority = app.workspace_compatibility.authority().cloned();
            match confirmed_raw_execution_request(
                &app.raw_mode,
                builtin_raw_catalog(),
                authority.as_ref(),
                request_id,
            ) {
                Ok(request) => {
                    reduce_raw_mode(
                        &mut app.raw_mode,
                        builtin_raw_catalog(),
                        authority.as_ref(),
                        RawModeAction::OpenExecution(request.command.clone()),
                    );
                    return Some(Effect::StartRaw(request));
                }
                Err(error) => app.raw_mode.notification = Some(error.to_string()),
            }
        }
        Action::RawMode(RawModeAction::CancelExecution(request_id)) => {
            let cancellable = app
                .raw_mode
                .execution_states
                .get(&request_id)
                .is_some_and(|state| !state.phase.is_terminal() && !state.cancellation_requested);
            if cancellable {
                return Some(Effect::CancelRaw(request_id));
            }
            app.raw_mode.notification =
                Some("Raw execution is unknown, terminal, or already cancelling.".into());
        }
        Action::RawMode(RawModeAction::SetExecutionAttachment {
            request,
            attachment,
        }) => {
            let available = app
                .raw_mode
                .execution_states
                .get(&request)
                .is_some_and(|state| !state.phase.is_terminal() && state.attachment != attachment);
            if available {
                return Some(Effect::SetRawAttachment {
                    request,
                    attached: attachment == RawAttachmentState::Attached,
                });
            }
            app.raw_mode.notification = Some(
                "Raw execution is terminal, unknown, or already has that attachment state.".into(),
            );
        }
        Action::RawMode(RawModeAction::CloseExecution) => {
            let detach = app
                .raw_mode
                .selected_execution()
                .filter(|state| {
                    !state.phase.is_terminal() && state.attachment == RawAttachmentState::Attached
                })
                .map(|state| state.request.id.clone());
            let authority = app.workspace_compatibility.authority().cloned();
            reduce_raw_mode(
                &mut app.raw_mode,
                builtin_raw_catalog(),
                authority.as_ref(),
                RawModeAction::CloseExecution,
            );
            if let Some(request) = detach {
                return Some(Effect::SetRawAttachment {
                    request,
                    attached: false,
                });
            }
        }
        Action::RawMode(action) => {
            let authority = app.workspace_compatibility.authority().cloned();
            let persist_favorites = matches!(
                action,
                RawModeAction::ToggleFavorite
                    | RawModeAction::RenameFavorite { .. }
                    | RawModeAction::MoveFavorite { .. }
                    | RawModeAction::RemoveFavorite
                    | RawModeAction::ConfirmFavorite
            );
            reduce_raw_mode(
                &mut app.raw_mode,
                builtin_raw_catalog(),
                authority.as_ref(),
                action,
            );
            if persist_favorites {
                return Some(Effect::PersistSettings);
            }
        }
        Action::EditActivePopup(command) => {
            let (editor, validation_error) = match app.active_dialog_mut() {
                Some(
                    Dialog::BuildEnvironmentEditor(editor)
                    | Dialog::BuildEnvironmentCloneEditor(editor)
                    | Dialog::BbmaskEdit(editor)
                    | Dialog::SdkPublishTomlEditor(editor)
                    | Dialog::SdkNativeTomlEditor(editor),
                ) => (editor, None),
                Some(Dialog::ConfigEdit { editor, .. }) => (editor, None),
                Some(Dialog::BuildTarget { editor, .. }) => (editor, None),
                Some(Dialog::WicCreateTomlEditor {
                    editor,
                    validation_error,
                })
                | Some(Dialog::TestLaunchTomlEditor {
                    editor,
                    validation_error,
                    ..
                })
                | Some(Dialog::TestResultImportTomlEditor {
                    editor,
                    validation_error,
                })
                | Some(Dialog::TestComparisonTomlEditor {
                    editor,
                    validation_error,
                })
                | Some(Dialog::TestJunitTomlEditor {
                    editor,
                    validation_error,
                    ..
                }) => (editor, Some(validation_error)),
                Some(Dialog::Security(SecurityDialog::Import {
                    editor,
                    validation_error,
                })) => (editor, Some(validation_error)),
                Some(Dialog::Qa(QaDialog::Import {
                    editor,
                    validation_error,
                })) => (editor, Some(validation_error)),
                Some(Dialog::Maintenance(dialog)) => match dialog.as_mut() {
                    MaintenanceDialog::ReadinessToml {
                        editor,
                        validation_error,
                    }
                    | MaintenanceDialog::CleanupToml {
                        editor,
                        validation_error,
                    }
                    | MaintenanceDialog::PrServiceToml {
                        editor,
                        validation_error,
                        ..
                    }
                    | MaintenanceDialog::LockedCacheToml {
                        editor,
                        validation_error,
                    }
                    | MaintenanceDialog::BuildHistoryToml {
                        editor,
                        validation_error,
                    }
                    | MaintenanceDialog::GitArchiveToml {
                        editor,
                        validation_error,
                    } => (editor, Some(validation_error)),
                    _ => return None,
                },
                _ => return None,
            };
            match command {
                PopupEditorCommand::ToggleInsert => editor.toggle_insert(),
                PopupEditorCommand::ToggleVisual => {
                    let mode = if editor.mode() == TextAreaMode::Visual {
                        TextAreaMode::Normal
                    } else {
                        TextAreaMode::Visual
                    };
                    editor.set_mode(mode);
                }
                PopupEditorCommand::Insert(character)
                    if editor.editing
                        && !character.is_control()
                        && editor.text.len() + character.len_utf8() <= 16_384 =>
                {
                    editor.insert(&character.to_string());
                    if let Some(error) = validation_error {
                        *error = None;
                    }
                }
                PopupEditorCommand::Insert(_) => {}
                PopupEditorCommand::Newline if editor.editing => editor.insert("\n"),
                PopupEditorCommand::Newline => {}
                PopupEditorCommand::Backspace if editor.editing => {
                    editor.backspace();
                    if let Some(error) = validation_error {
                        *error = None;
                    }
                }
                PopupEditorCommand::Backspace => {}
                PopupEditorCommand::Delete => editor.delete_forward(),
                PopupEditorCommand::Left => editor.left(),
                PopupEditorCommand::Right => editor.right(),
                PopupEditorCommand::WordLeft => editor.move_cursor(TextAreaMotion::WordLeft),
                PopupEditorCommand::WordRight => editor.move_cursor(TextAreaMotion::WordRight),
                PopupEditorCommand::Up => editor.up(),
                PopupEditorCommand::Down => editor.down(),
                PopupEditorCommand::Home => editor.home(),
                PopupEditorCommand::End => editor.end(),
                PopupEditorCommand::PageUp => editor.move_cursor(TextAreaMotion::PageUp),
                PopupEditorCommand::PageDown => editor.move_cursor(TextAreaMotion::PageDown),
                PopupEditorCommand::Undo => {
                    editor.undo();
                }
                PopupEditorCommand::Redo => {
                    editor.redo();
                }
                PopupEditorCommand::SelectPosition {
                    line,
                    column,
                    extend,
                } => editor.select_position(line, column, extend),
                PopupEditorCommand::PasteText { text, source } if editor.editing => {
                    if let Err(error) = editor.paste_text(&text, source) {
                        app.notification = Some(format!("Editor paste rejected: {error:?}"));
                    } else if let Some(error) = validation_error {
                        *error = None;
                    }
                }
                PopupEditorCommand::PasteText { .. } => {}
                PopupEditorCommand::SelectValue => match editor.select_toml_value_at_cursor() {
                    Ok(()) => editor.set_mode(TextAreaMode::Insert),
                    Err(message) => app.notification = Some(message),
                },
                PopupEditorCommand::Copy => {
                    return Some(Effect::CopyToClipboard(editor.copy_selection_or_line()));
                }
                PopupEditorCommand::Paste if editor.editing => {
                    editor.paste();
                    if let Some(error) = validation_error {
                        *error = None;
                    }
                }
                PopupEditorCommand::Paste => {}
            }
        }
        Action::OpenBuildEnvironmentCloneEditor => {
            let mut editor = PopupEditor::new(
                "repository = \"\"\ndestination = \"\"\nrevision = \"\"\nbuild = \"\"\n".into(),
            );
            let _ = editor.select_toml_value("repository");
            open_dialog(app, Dialog::BuildEnvironmentCloneEditor(editor));
        }
        Action::ToggleBuildEnvironmentCloneEditor => {
            if let Some(Dialog::BuildEnvironmentCloneEditor(editor)) = app.active_dialog_mut() {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendBuildEnvironmentCloneEditor(character) => {
            if let Some(Dialog::BuildEnvironmentCloneEditor(editor)) = app.active_dialog_mut()
                && editor.editing
            {
                editor.insert(&character.to_string());
            }
        }
        Action::BackspaceBuildEnvironmentCloneEditor => {
            if let Some(Dialog::BuildEnvironmentCloneEditor(editor)) = app.active_dialog_mut()
                && editor.editing
            {
                editor.backspace();
            }
        }
        Action::ReviewBuildEnvironmentClone => {
            if let Some(Dialog::BuildEnvironmentCloneEditor(editor)) = app.active_dialog().cloned()
            {
                let mut values = HashMap::new();
                for line in editor.text.lines() {
                    if let Some((name, value)) = line.split_once('=') {
                        values.insert(
                            name.trim().to_owned(),
                            value.trim().trim_matches('"').to_owned(),
                        );
                    }
                }
                let plan = BuildEnvironmentClonePlan {
                    request: BuildEnvironmentCloneRequest {
                        repository: values.remove("repository").unwrap_or_default(),
                        destination: PathBuf::from(
                            values.remove("destination").unwrap_or_default(),
                        ),
                        revision: values.remove("revision").filter(|value| !value.is_empty()),
                    },
                    build_dir: PathBuf::from(values.remove("build").unwrap_or_default()),
                };
                match plan.validate() {
                    Ok(()) => replace_dialog(app, Dialog::BuildEnvironmentCloneReview(plan)),
                    Err(error) => app.notification = Some(error.to_string()),
                }
            }
        }
        Action::ConfirmBuildEnvironmentClone => {
            if app
                .background_activities
                .contains(&BackgroundActivity::Cloning)
            {
                return None;
            }
            if let Some(Dialog::BuildEnvironmentCloneReview(plan)) = app.active_dialog().cloned() {
                close_dialog(app);
                return Some(Effect::CloneBuildEnvironment(plan));
            }
        }
        Action::CancelBuildEnvironmentClone => {
            if matches!(
                app.active_dialog(),
                Some(
                    Dialog::BuildEnvironmentCloneEditor(_) | Dialog::BuildEnvironmentCloneReview(_)
                )
            ) {
                close_dialog(app);
            }
        }
        Action::EnvironmentSetup(action) => return environment_setup_update(app, action),
        Action::OpenBuildEnvironmentEditor => {
            let profile = match &app.build_environment {
                BuildEnvironmentState::Configured(profile)
                | BuildEnvironmentState::Connected(profile)
                | BuildEnvironmentState::Failed { profile, .. }
                | BuildEnvironmentState::Verifying { profile, .. } => Some(profile),
                BuildEnvironmentState::Unconfigured => None,
            };
            let value = |name: &str, path: Option<&Path>| {
                format!(
                    "{name} = \"{}\"",
                    path.map_or_else(String::new, |p| p.display().to_string())
                )
            };
            let content = format!(
                "{}\n{}\n{}\n",
                value("source", profile.map(|p| p.source_dir.as_path())),
                value("build", profile.map(|p| p.build_dir.as_path())),
                value("script", profile.map(|p| p.init_script.as_path()))
            );
            let mut editor = PopupEditor::new(content);
            let _ = editor.select_toml_value("source");
            open_dialog(app, Dialog::BuildEnvironmentEditor(editor));
        }
        Action::ToggleBuildEnvironmentEditor => {
            if let Some(Dialog::BuildEnvironmentEditor(editor)) = app.active_dialog_mut() {
                editor.editing = !editor.editing;
            }
        }
        Action::AppendBuildEnvironmentEditor(character) => {
            if let Some(Dialog::BuildEnvironmentEditor(editor)) = app.active_dialog_mut()
                && editor.editing
            {
                editor.insert(&character.to_string());
            }
        }
        Action::BackspaceBuildEnvironmentEditor => {
            if let Some(Dialog::BuildEnvironmentEditor(editor)) = app.active_dialog_mut()
                && editor.editing
            {
                editor.backspace();
            }
        }
        Action::ApplyBuildEnvironmentEditor => {
            if let Some(Dialog::BuildEnvironmentEditor(editor)) = app.active_dialog().cloned() {
                let mut values = HashMap::new();
                for line in editor.text.lines() {
                    if let Some((name, value)) = line.split_once('=') {
                        let value = value.trim().trim_matches('"').to_owned();
                        values.insert(name.trim().to_owned(), value);
                    }
                }
                let profile = BuildEnvironmentProfile {
                    source_dir: PathBuf::from(values.remove("source").unwrap_or_default()),
                    build_dir: PathBuf::from(values.remove("build").unwrap_or_default()),
                    init_script: PathBuf::from(values.remove("script").unwrap_or_default()),
                };
                close_dialog(app);
                return update(app, Action::ConfigureBuildEnvironment(profile));
            }
        }
        Action::CloseBuildEnvironmentEditor => {
            if matches!(app.active_dialog(), Some(Dialog::BuildEnvironmentEditor(_))) {
                close_dialog(app);
            }
        }
        Action::OpenThemePicker => {
            let selection = THEMES
                .iter()
                .position(|theme| *theme == app.theme)
                .unwrap_or(0);
            open_dialog(
                app,
                Dialog::ThemePicker {
                    selection,
                    original_theme: app.theme,
                    original_color_enabled: app.color_enabled,
                    original_settings_dirty: app.settings_dirty,
                },
            );
        }
        Action::SelectTheme { delta } => {
            if let Some(Dialog::ThemePicker { selection, .. }) = app.active_dialog_mut() {
                *selection = if delta.is_negative() {
                    selection.saturating_sub(delta.unsigned_abs())
                } else {
                    selection
                        .saturating_add(delta as usize)
                        .min(THEMES.len() - 1)
                };
                app.theme = THEMES[*selection];
                if !app.color_forced_off {
                    app.color_enabled = true;
                }
                app.settings_dirty = true;
            }
        }
        Action::ApplySelectedTheme => {
            if let Some(Dialog::ThemePicker { selection, .. }) = app.active_dialog().cloned() {
                app.theme = THEMES[selection.min(THEMES.len() - 1)];
                if !app.color_forced_off {
                    app.color_enabled = true;
                }
                app.settings_dirty = true;
                close_dialog(app);
                return Some(Effect::PersistSettings);
            }
        }
        Action::CloseThemePicker => {
            if let Some(Dialog::ThemePicker {
                original_theme,
                original_color_enabled,
                original_settings_dirty,
                ..
            }) = app.active_dialog().cloned()
            {
                app.theme = original_theme;
                app.color_enabled = original_color_enabled;
                app.settings_dirty = original_settings_dirty;
                close_dialog(app);
            }
        }
        Action::ConfigureBuildEnvironment(profile) => match profile.validate() {
            Ok(()) => {
                app.build_environment = BuildEnvironmentState::Configured(profile);
                app.available_images.clear();
                app.notification = None;
            }
            Err(error) => app.notification = Some(error.to_string()),
        },
        Action::BeginBuildEnvironmentEdit => {
            let profile = match &app.build_environment {
                BuildEnvironmentState::Configured(profile)
                | BuildEnvironmentState::Connected(profile)
                | BuildEnvironmentState::Failed { profile, .. }
                | BuildEnvironmentState::Verifying { profile, .. } => Some(profile),
                BuildEnvironmentState::Unconfigured => None,
            };
            app.build_environment_draft = Some(BuildEnvironmentDraft {
                source: profile.map_or_else(String::new, |p| p.source_dir.display().to_string()),
                build: profile.map_or_else(String::new, |p| p.build_dir.display().to_string()),
                script: profile.map_or_else(String::new, |p| p.init_script.display().to_string()),
                field: BuildEnvironmentField::Source,
                editing: true,
            });
        }
        Action::SelectBuildEnvironmentField { delta } => {
            if let Some(draft) = app.build_environment_draft.as_mut() {
                let index: usize = match draft.field {
                    BuildEnvironmentField::Source => 0,
                    BuildEnvironmentField::Build => 1,
                    BuildEnvironmentField::Script => 2,
                };
                let next = if delta.is_negative() {
                    index.saturating_sub(delta.unsigned_abs())
                } else {
                    index.saturating_add(delta as usize).min(2)
                };
                draft.field = match next {
                    0 => BuildEnvironmentField::Source,
                    1 => BuildEnvironmentField::Build,
                    _ => BuildEnvironmentField::Script,
                };
            }
        }
        Action::AppendBuildEnvironmentField(character) => {
            if let Some(draft) = app.build_environment_draft.as_mut() {
                let value = match draft.field {
                    BuildEnvironmentField::Source => &mut draft.source,
                    BuildEnvironmentField::Build => &mut draft.build,
                    BuildEnvironmentField::Script => &mut draft.script,
                };
                value.push(character);
            }
        }
        Action::BackspaceBuildEnvironmentField => {
            if let Some(draft) = app.build_environment_draft.as_mut() {
                let value = match draft.field {
                    BuildEnvironmentField::Source => &mut draft.source,
                    BuildEnvironmentField::Build => &mut draft.build,
                    BuildEnvironmentField::Script => &mut draft.script,
                };
                value.pop();
            }
        }
        Action::FinishBuildEnvironmentEdit => {
            if let Some(draft) = app.build_environment_draft.as_mut() {
                draft.editing = false;
            }
        }
        Action::CancelBuildEnvironmentEdit => app.build_environment_draft = None,
        Action::ApplyBuildEnvironmentProfile => {
            if let Some(draft) = app.build_environment_draft.take() {
                let profile = BuildEnvironmentProfile {
                    source_dir: PathBuf::from(draft.source),
                    build_dir: PathBuf::from(draft.build),
                    init_script: PathBuf::from(draft.script),
                };
                let _ = update(app, Action::ConfigureBuildEnvironment(profile));
            }
        }
        Action::BeginBuildEnvironmentVerification => {
            let profile = match &app.build_environment {
                BuildEnvironmentState::Configured(profile)
                | BuildEnvironmentState::Failed { profile, .. } => profile.clone(),
                BuildEnvironmentState::Verifying { .. } => return None,
                BuildEnvironmentState::Unconfigured | BuildEnvironmentState::Connected(_) => {
                    app.notification =
                        Some("Select a build environment before verification.".into());
                    return None;
                }
            };
            app.build_environment_generation = app.build_environment_generation.wrapping_add(1);
            let generation = app.build_environment_generation;
            app.build_environment = BuildEnvironmentState::Verifying {
                profile: profile.clone(),
                generation,
            };
            return Some(Effect::VerifyBuildEnvironment {
                profile,
                generation,
            });
        }
        Action::BuildEnvironmentVerified { generation } => {
            if let BuildEnvironmentState::Verifying {
                profile,
                generation: pending,
            } = &app.build_environment
                && *pending == generation
            {
                app.build_environment = BuildEnvironmentState::Connected(profile.clone());
                app.notification = None;
            }
        }
        Action::BuildEnvironmentVerificationFailed {
            generation,
            message,
        } => {
            if let BuildEnvironmentState::Verifying {
                profile,
                generation: pending,
            } = &app.build_environment
                && *pending == generation
            {
                app.build_environment = BuildEnvironmentState::Failed {
                    profile: profile.clone(),
                    message: message.clone(),
                };
                app.notification = Some(format!("BitBake connection failed: {message}"));
            }
        }
        Action::CycleFocus { backwards } => {
            if matches!(app.focus, FocusTarget::Dialog | FocusTarget::CommandPalette) {
                return None;
            }
            let targets = pane_focus_targets(app).collect::<Vec<_>>();
            let current = targets
                .iter()
                .position(|target| *target == app.focus)
                .unwrap_or(0);
            let next = if backwards {
                (current + targets.len() - 1) % targets.len()
            } else {
                (current + 1) % targets.len()
            };
            app.focus = targets[next];
            if app.zoomed_pane.is_some() {
                app.zoomed_pane = Some(app.focus);
            }
        }
        Action::CyclePaneSubfocus { backwards } => match app.focus {
            FocusTarget::Workspace => {
                let count = workspace_subfocus_count(app.screen);
                let current = match app.workspace_subfocus {
                    WorkspaceSubfocus::Main => 0,
                    WorkspaceSubfocus::Secondary => 1,
                    WorkspaceSubfocus::Context => 2,
                }
                .min(count.saturating_sub(1));
                let next = if backwards {
                    (current + count - 1) % count
                } else {
                    (current + 1) % count
                };
                app.workspace_subfocus = match next {
                    0 => WorkspaceSubfocus::Main,
                    1 => WorkspaceSubfocus::Secondary,
                    _ => WorkspaceSubfocus::Context,
                };
            }
            FocusTarget::Inspector => {
                let current = match app.inspector_subfocus {
                    InspectorSubfocus::Facts => 0,
                    InspectorSubfocus::Output => 1,
                    InspectorSubfocus::Actions => 2,
                };
                let next = if backwards {
                    (current + 2) % 3
                } else {
                    (current + 1) % 3
                };
                app.inspector_subfocus = match next {
                    0 => InspectorSubfocus::Facts,
                    1 => InspectorSubfocus::Output,
                    _ => InspectorSubfocus::Actions,
                };
            }
            FocusTarget::Navigator | FocusTarget::Dialog | FocusTarget::CommandPalette => {}
        },
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
