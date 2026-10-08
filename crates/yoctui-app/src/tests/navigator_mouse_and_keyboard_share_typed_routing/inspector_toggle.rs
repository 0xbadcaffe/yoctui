use super::*;

#[test]
fn inspector_toggle_routes_globally_and_keeps_modal_focus_trapped() {
    let mut app = yoctui_model::App::new(16, 4096);
    for screen in [
        Screen::Dashboard,
        Screen::Insights,
        Screen::Tasks,
        Screen::BuildHistory,
        Screen::Dependencies,
        Screen::Signatures,
        Screen::LayerRelationships,
        Screen::Recipes,
        Screen::Devtool,
        Screen::Packages,
        Screen::Images,
        Screen::Hardware,
        Screen::Kernel,
        Screen::Firmware,
        Screen::Sdk,
        Screen::Testing,
        Screen::Security,
        Screen::Qa,
        Screen::Layers,
        Screen::Configuration,
        Screen::Bbmask,
        Screen::RawMode,
        Screen::TerminalSessions,
        Screen::Maintenance,
        Screen::Logs,
        Screen::Errors,
        Screen::Help,
        Screen::BuildEnvironment,
        Screen::Compatibility,
        Screen::Settings,
    ] {
        app.screen = screen;
        assert_eq!(
            crate::inspector_toggle_action(&app, Input::Alt('i')),
            Some(Action::ToggleInspector)
        );
    }
    app.focus = FocusTarget::Dialog;
    assert_eq!(crate::inspector_toggle_action(&app, Input::Alt('i')), None);
    app.onboarding.open = true;
    let _ = yoctui_model::update(&mut app, Action::ToggleInspector);
    assert!(
        !app.inspector_visible,
        "modal reducer must also trap layout changes"
    );
    app.focus = FocusTarget::Workspace;
    app.onboarding.open = true;
    assert_eq!(crate::inspector_toggle_action(&app, Input::Alt('i')), None);
    app.onboarding.open = false;
    app.command_palette_open = true;
    assert_eq!(crate::inspector_toggle_action(&app, Input::Alt('i')), None);
}

#[test]
fn hidden_inspector_mouse_region_belongs_to_expanded_workspace() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = Screen::Tasks;
    for (width, height) in [(130, 24), (140, 30), (160, 50), (200, 60)] {
        app.inspector_visible = true;
        let before = workbench_pane_widths(&app, width, height);
        let _ = yoctui_model::update(&mut app, Action::ToggleInspector);
        assert_eq!(
            workbench_pane_widths(&app, width, height),
            [before[0], before[1] + before[2], 0]
        );
        assert_eq!(
            mouse_action_for_app(
                MouseInput {
                    kind: MouseKind::Down,
                    column: width - 2,
                    row: 5
                },
                &app,
                width,
                height
            ),
            Some(Action::Focus(FocusTarget::Workspace))
        );
    }
    app.focus = FocusTarget::Inspector;
    app.zoomed_pane = Some(FocusTarget::Inspector);
    app.inspector_visible = true;
    let _ = yoctui_model::update(&mut app, Action::ToggleInspector);
    assert_eq!(app.focus, FocusTarget::Workspace);
    assert_eq!(app.zoomed_pane, None);
}

#[test]
fn inspector_toggle_respects_custom_key_and_survives_screen_navigation() {
    use yoctui_model::{EffectiveKeymap, KeymapScope, OperatorActionId};
    let mut app = yoctui_model::App::new(16, 4096);
    app.keymap_preferences = app.keymap_preferences.with_action_sequences(
        OperatorActionId::new("view.toggle-inspector"),
        KeymapScope::Global,
        vec!["Alt+j".parse().unwrap()],
    );
    app.effective_keymap = EffectiveKeymap::from_preferences(&app.keymap_preferences).unwrap();
    assert_eq!(crate::inspector_toggle_action(&app, Input::Alt('i')), None);
    assert_eq!(
        crate::inspector_toggle_action(&app, Input::Alt('j')),
        Some(Action::ToggleInspector)
    );
    let _ = yoctui_model::update(&mut app, Action::ToggleInspector);
    let _ = yoctui_model::update(&mut app, Action::Open(Screen::Images));
    assert!(app.inspector_visible);
}

#[test]
fn preferences_reset_shortcut_routes_to_selected_setting() {
    assert_eq!(
        crate::settings_action(Input::Backspace),
        Some(Action::ResetSelectedPreference)
    );
    assert_eq!(
        crate::settings_action(Input::Alt('r')),
        Some(Action::ResetPreferences)
    );
}
