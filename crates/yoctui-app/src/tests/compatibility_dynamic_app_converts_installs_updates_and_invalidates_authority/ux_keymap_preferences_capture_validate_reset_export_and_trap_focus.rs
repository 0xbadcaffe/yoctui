use super::*;

#[test]
fn ux_keymap_preferences_capture_validate_reset_export_and_trap_focus() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = yoctui_model::Screen::Settings;
    app.settings_selection = yoctui_model::SETTINGS.len() - 1;
    let action = settings_action(Input::Enter).unwrap();
    assert_eq!(compatibility_workspace_action(&mut app, action), None);
    assert!(app.keymap_preferences_ui.open);
    assert_eq!(app.focus, yoctui_model::FocusTarget::Dialog);
    assert_eq!(
        keymap_action_for_app(&mut app, Input::Char('l')),
        KeymapInputResult::Unmatched
    );

    let action = keymap_preferences_action(&app, Input::Char('/')).unwrap();
    let _ = compatibility_workspace_action(&mut app, action);
    for character in "navigate.logs".chars() {
        let action = keymap_preferences_action(&app, Input::Char(character)).unwrap();
        let _ = compatibility_workspace_action(&mut app, action);
    }
    let action = keymap_preferences_action(&app, Input::Enter).unwrap();
    let _ = compatibility_workspace_action(&mut app, action);
    assert_eq!(
        yoctui_model::keymap_preference_rows(
            &app.keymap_preferences,
            &app.effective_keymap,
            &app.keymap_preferences_ui.query,
        )
        .len(),
        1
    );

    let action = keymap_preferences_action(&app, Input::Enter).unwrap();
    let _ = compatibility_workspace_action(&mut app, action);
    let action = keymap_preferences_action(&app, Input::Char('e')).unwrap();
    let _ = compatibility_workspace_action(&mut app, action);
    let action = keymap_preferences_action(&app, Input::CtrlS).unwrap();
    assert_eq!(compatibility_workspace_action(&mut app, action), None);
    assert!(
        app.keymap_preferences_ui
            .validation_error
            .as_deref()
            .is_some_and(|error| error.contains("collision"))
    );

    let action = keymap_preferences_action(&app, Input::Backspace).unwrap();
    let _ = compatibility_workspace_action(&mut app, action);
    let action = keymap_preferences_action(&app, Input::Char('z')).unwrap();
    let _ = compatibility_workspace_action(&mut app, action);
    let action = keymap_preferences_action(&app, Input::CtrlS).unwrap();
    assert_eq!(
        compatibility_workspace_action(&mut app, action),
        Some(yoctui_model::Effect::PersistSettings)
    );
    assert!(app.settings_dirty);
    assert!(app.keymap_preferences_ui.capture.is_none());
    assert!(app.effective_keymap.bindings().iter().any(|binding| {
        binding.action_id.as_str() == "navigate.logs"
            && binding.sequence.to_string() == "z"
            && !binding.is_default
    }));

    let action = keymap_preferences_action(&app, Input::Char('e')).unwrap();
    let Some(yoctui_model::Effect::CopyToClipboard(report)) =
        compatibility_workspace_action(&mut app, action)
    else {
        panic!("export should use the bounded clipboard effect")
    };
    assert!(report.contains("navigate.logs\tz\tcustom"));

    let _ = compatibility_workspace_action(
        &mut app,
        Action::SettingsPersistenceFailed("read-only filesystem".into()),
    );
    let action = keymap_preferences_action(&app, Input::Char('p')).unwrap();
    assert_eq!(
        compatibility_workspace_action(&mut app, action),
        Some(yoctui_model::Effect::PersistSettings)
    );

    let _ = compatibility_workspace_action(&mut app, Action::ClearKeymapPreferenceQuery);
    let _ = compatibility_workspace_action(&mut app, Action::BeginKeymapPreferenceSearch);
    for character in "help.open".chars() {
        let _ = compatibility_workspace_action(
            &mut app,
            Action::AppendKeymapPreferenceQuery(character),
        );
    }
    let _ = compatibility_workspace_action(&mut app, Action::FinishKeymapPreferenceSearch);
    assert_eq!(
        compatibility_workspace_action(&mut app, Action::RemoveKeymapBinding),
        None
    );
    assert!(
        app.keymap_preferences_ui
            .validation_error
            .as_deref()
            .is_some_and(|error| error.contains("critical action help.open"))
    );

    assert_eq!(
        compatibility_workspace_action(&mut app, Action::ResetAllKeymapBindings),
        Some(yoctui_model::Effect::PersistSettings)
    );
    assert!(app.keymap_preferences.overrides.is_empty());
    let _ = compatibility_workspace_action(&mut app, Action::CloseKeymapPreferences);
    assert!(!app.keymap_preferences_ui.open);
    assert_eq!(app.focus, yoctui_model::FocusTarget::Navigator);
}
