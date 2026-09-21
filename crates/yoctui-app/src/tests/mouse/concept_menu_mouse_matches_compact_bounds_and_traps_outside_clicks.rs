use super::*;

#[test]
fn concept_menu_mouse_matches_compact_bounds_and_traps_outside_clicks() {
    for (width, height) in [(80, 24), (100, 30), (160, 50)] {
        let mut app = App::new(10, 1024);
        app.preferences.mouse_enabled = true;
        yoctui_model::update(&mut app, Action::OpenApplicationMenu);
        let (left, top, _, _) =
            application_menu_bounds(&app, width, height, app.active_menu_items().len()).unwrap();
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
