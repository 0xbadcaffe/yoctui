use super::*;

fn settings() -> App {
    let mut app = App::new(20, 4096);
    update(&mut app, Action::Open(Screen::Settings));
    app.settings_selection = SETTINGS
        .iter()
        .position(|setting| *setting == Setting::Editor)
        .unwrap();
    app
}

#[test]
fn editor_preferences_default_to_builtin_cancel_safely_and_persist_custom_argv() {
    let mut app = settings();
    assert!(app.preferences.terminal_editor.is_none());
    update(&mut app, Action::ChangeSelectedSetting { backwards: false });
    assert_eq!(app.editor_settings.draft.as_deref(), Some("builtin"));
    for ch in "nvim -u NONE".chars() {
        update(
            &mut app,
            Action::EditorSettings(EditorSettingsAction::Append(ch)),
        );
    }
    update(
        &mut app,
        Action::EditorSettings(EditorSettingsAction::Cancel),
    );
    assert!(app.preferences.terminal_editor.is_none());
    update(
        &mut app,
        Action::EditorSettings(EditorSettingsAction::Begin),
    );
    for ch in "nvim -u NONE".chars() {
        update(
            &mut app,
            Action::EditorSettings(EditorSettingsAction::Append(ch)),
        );
    }
    update(
        &mut app,
        Action::EditorSettings(EditorSettingsAction::TogglePlacement),
    );
    assert_eq!(
        update(&mut app, Action::EditorSettings(EditorSettingsAction::Save)),
        Some(Effect::PersistSettings)
    );
    assert_eq!(
        app.preferences.terminal_editor.as_deref(),
        Some("nvim -u NONE")
    );
    assert!(!app.preferences.terminal_editor_embedded);
    let serialized = toml::to_string(&app.effective_preferences()).unwrap();
    let restored: WorkbenchPreferences = toml::from_str(&serialized).unwrap();
    restored.validate().unwrap();
    assert_eq!(restored.terminal_editor, app.preferences.terminal_editor);
    assert!(!restored.terminal_editor_embedded);
    update(&mut app, Action::ResetSelectedPreference);
    assert!(app.preferences.terminal_editor.is_none());
    assert!(app.preferences.terminal_editor_embedded);
}

#[test]
fn editor_preferences_validate_bounded_commands_and_load_legacy_settings() {
    for invalid in ["", "vim\nother", "vim\0bad", &"x".repeat(4097)] {
        assert!(validate_editor_command(invalid).is_err());
    }
    let legacy: WorkbenchPreferences = toml::from_str("").unwrap();
    assert!(legacy.terminal_editor.is_none());
    assert!(legacy.terminal_editor_embedded);
}

#[test]
fn preferred_editor_terminal_handoff_preserves_workbench_and_rejects_dirty_buffer() {
    let mut app = App::new(20, 4096);
    app.screen = Screen::Devtool;
    update(
        &mut app,
        Action::OpenRecipeEditor {
            recipe: "busybox".into(),
            root: "/work/busybox".into(),
            files: vec!["Makefile".into()],
        },
    );
    update(&mut app, Action::LoadRecipeEditorContent("original".into()));
    let request = TerminalLaunchRequest {
        name: "Editor · Makefile · test".into(),
        kind: TerminalCreationKind::Utility,
        cwd: "/work/busybox".into(),
        program: "/usr/bin/vi".into(),
        arguments: vec!["/work/busybox/Makefile".into()],
        completion: Some(TerminalCompletion::ReturnEditor {
            path: "/work/busybox/Makefile".into(),
            name: "Editor · Makefile · test".into(),
        }),
    };
    assert!(matches!(
        update(
            &mut app,
            Action::EditorSettings(EditorSettingsAction::Launch(Box::new(request.clone())))
        ),
        Some(Effect::Terminal(TerminalEffect::Create { .. }))
    ));
    assert_eq!(app.screen, Screen::TerminalSessions);
    assert!(app.suspended_recipe_editor.is_some());
    assert!(app.active_dialog().is_none());
    update(&mut app, Action::RestoreRecipeEditor);
    assert_eq!(app.screen, Screen::Devtool);
    update(
        &mut app,
        Action::FocusRecipeEditor(RecipeEditorFocus::Document),
    );
    update(&mut app, Action::ToggleRecipeEditorEditing);
    update(
        &mut app,
        Action::EditRecipeEditor(PopupEditorCommand::Insert('x')),
    );
    assert!(matches!(app.active_dialog(), Some(Dialog::RecipeEditor(editor)) if editor.is_dirty()));
    assert_eq!(
        update(
            &mut app,
            Action::EditorSettings(EditorSettingsAction::Launch(Box::new(request)))
        ),
        None
    );
    assert!(app.suspended_recipe_editor.is_none());
}
