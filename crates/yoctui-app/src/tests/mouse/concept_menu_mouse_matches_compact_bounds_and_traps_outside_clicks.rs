use super::*;

#[test]
fn concept_menu_mouse_matches_compact_bounds_and_traps_outside_clicks() {
    for (width, height, expected_width, expected_height) in
        [(80, 24, 76, 18), (100, 30, 96, 24), (160, 50, 100, 28)]
    {
        let mut app = App::new(10, 1024);
        app.preferences.mouse_enabled = true;
        yoctui_model::update(&mut app, Action::OpenApplicationMenu);
        let (left, top, menu_width, menu_height) =
            application_menu_bounds(&app, width, height, app.active_menu_items().len()).unwrap();
        assert_eq!((menu_width, menu_height), (expected_width, expected_height));
        assert!(left >= 2);
        assert!(left + menu_width + 2 <= width);
        let action = mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: left + 13,
                row: top + 1,
            },
            &app,
            width,
            height,
        )
        .unwrap();
        yoctui_model::update(&mut app, action);
        assert_eq!(app.menu.group(), yoctui_model::ApplicationMenuGroup::Build);
        assert!(
            mouse_action_for_app(
                MouseInput {
                    kind: MouseKind::Down,
                    column: 0,
                    row: 0
                },
                &app,
                width,
                height
            )
            .is_none()
        );
        let action = mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: left + 2,
                row: top + 4,
            },
            &app,
            width,
            height,
        )
        .unwrap();
        yoctui_model::update(&mut app, action);
        assert_eq!(app.menu.item_selection, 1);
        assert_eq!(app.focus, FocusTarget::Dialog);
    }
}
