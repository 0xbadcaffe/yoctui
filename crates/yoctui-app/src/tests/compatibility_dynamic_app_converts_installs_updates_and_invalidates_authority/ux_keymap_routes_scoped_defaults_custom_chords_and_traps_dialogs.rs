use super::*;

#[test]
fn ux_keymap_routes_scoped_defaults_custom_chords_and_traps_dialogs() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = yoctui_model::Screen::Images;
    assert!(matches!(
        keymap_action_for_app(&mut app, Input::Char('i')),
        KeymapInputResult::Action(action) if matches!(*action, Action::OpenImagePicker(_))
    ));

    app.install_keymap(yoctui_model::KeymapPreferences {
        schema_version: yoctui_model::KEYMAP_SCHEMA_VERSION,
        overrides: vec![yoctui_model::KeymapOverride {
            action_id: "navigate.logs".into(),
            scope: yoctui_model::KeymapScope::Global,
            sequences: vec!["z".parse().unwrap(), "g l".parse().unwrap()],
        }],
    })
    .unwrap();
    app.screen = yoctui_model::Screen::Dashboard;
    assert_eq!(
        keymap_action_for_app(&mut app, Input::Char('l')),
        KeymapInputResult::Unmatched,
        "an overridden default must not leak through the legacy router"
    );
    assert_eq!(
        keymap_action_for_app(&mut app, Input::Char('z')),
        KeymapInputResult::Action(Box::new(Action::Open(yoctui_model::Screen::Logs)))
    );
    assert_eq!(
        keymap_action_for_app(&mut app, Input::Char('g')),
        KeymapInputResult::Pending
    );
    assert_eq!(
        keymap_action_for_app(&mut app, Input::Char('l')),
        KeymapInputResult::Action(Box::new(Action::Open(yoctui_model::Screen::Logs)))
    );

    app.dialogs
        .push_back(yoctui_model::Dialog::QuitConfirmation);
    app.focus = yoctui_model::FocusTarget::Dialog;
    assert_eq!(
        keymap_action_for_app(&mut app, Input::Char('z')),
        KeymapInputResult::Unmatched
    );
    assert!(!app.keymap_chord.is_pending());
}
