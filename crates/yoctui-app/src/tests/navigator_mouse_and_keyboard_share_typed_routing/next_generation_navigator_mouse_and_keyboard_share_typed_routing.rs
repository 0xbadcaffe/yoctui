use super::*;

#[test]
fn next_generation_navigator_mouse_and_keyboard_share_typed_routing() {
    let mut app = yoctui_model::App::new(16, 4096);
    let click_layers = MouseInput {
        kind: MouseKind::Down,
        column: 5,
        row: 7,
    };
    let select = mouse_action_for_app(click_layers, &app, 180, 40);
    assert_eq!(select, Some(Action::SelectNavigatorAt { index: 2 }));
    let _ = yoctui_model::update(&mut app, select.unwrap());
    assert_eq!(app.navigator_selection, 2);
    assert_eq!(app.focus, FocusTarget::Navigator);
    assert_eq!(
        mouse_action_for_app(click_layers, &app, 180, 40),
        Some(Action::ActivateNavigator)
    );

    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::Left),
        Some(Action::CollapseNavigatorGroup)
    );
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::Right),
        Some(Action::ExpandNavigatorGroup)
    );
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::Char('h')),
        Some(Action::CollapseNavigatorGroup)
    );
    assert_eq!(
        focus_action(FocusTarget::Navigator, Input::Char('l')),
        Some(Action::ExpandNavigatorGroup)
    );

    let content_heading = MouseInput {
        kind: MouseKind::Down,
        column: 5,
        row: 6,
    };
    assert_eq!(
        mouse_action_for_app(content_heading, &app, 180, 40),
        Some(Action::ToggleNavigatorGroup { group: 1 })
    );
    let collapse = mouse_action_for_app(content_heading, &app, 180, 40).unwrap();
    let _ = yoctui_model::update(&mut app, collapse);
    assert!(!app.navigator_groups_expanded[1]);
    assert_eq!(app.navigator_selection, 2);
    assert_eq!(
        mouse_action_for_app(content_heading, &app, 180, 40),
        Some(Action::ToggleNavigatorGroup { group: 1 })
    );
    let reopen = mouse_action_for_app(content_heading, &app, 180, 40).unwrap();
    let _ = yoctui_model::update(&mut app, reopen);
    assert!(app.navigator_groups_expanded[1]);
}
