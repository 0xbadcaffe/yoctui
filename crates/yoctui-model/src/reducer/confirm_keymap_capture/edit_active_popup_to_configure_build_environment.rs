use super::*;

pub(super) fn reduce_actions(app: &mut App, action: Action) -> Option<Effect> {
    match action {
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
        _ => unreachable!("action routed to the wrong reducer"),
    }
    synchronize_focus(app);
    None
}
