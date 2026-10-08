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
        assert_eq!(left, (width - menu_width) / 2);
        assert_eq!(top, (height - menu_height) / 2);
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

#[test]
fn application_menu_stays_centered_across_resize_and_chrome_preferences() {
    let mut app = App::new(10, 1024);
    yoctui_model::update(&mut app, Action::OpenApplicationMenu);
    for inspector in [false, true] {
        app.set_inspector_visible(inspector);
        for (width, height) in [(64, 16), (80, 24), (101, 31), (160, 50), (240, 80)] {
            let (left, top, menu_width, menu_height) =
                application_menu_bounds(&app, width, height, app.active_menu_items().len())
                    .unwrap();
            assert!((left as i32 - (width - left - menu_width) as i32).abs() <= 1);
            assert!((top as i32 - (height - top - menu_height) as i32).abs() <= 1);
        }
    }
    assert!(application_menu_bounds(&app, 63, 16, 0).is_none());
    assert!(application_menu_bounds(&app, 64, 15, 0).is_none());
}

#[test]
fn every_menu_group_mouse_hit_matches_the_rendered_order_on_laptop_terminals() {
    let mut app = App::new(10, 1024);
    yoctui_model::update(&mut app, Action::OpenApplicationMenu);
    for (width, height) in [(80, 24), (100, 30), (160, 50)] {
        let (left, top, menu_width, _) =
            application_menu_bounds(&app, width, height, app.active_menu_items().len()).unwrap();
        let mut column = left + 1;
        for (index, group) in yoctui_model::ApplicationMenuGroup::ALL.iter().enumerate() {
            let middle = column + 1 + group.label().len() as u16 / 2;
            assert!(middle < left + menu_width - 1);
            let action = mouse_action_for_app(
                MouseInput {
                    kind: MouseKind::Down,
                    column: middle,
                    row: top + 1,
                },
                &app,
                width,
                height,
            )
            .unwrap();
            yoctui_model::update(&mut app, action);
            assert_eq!(app.menu.group_selection, index);
            assert_eq!(app.menu.group(), *group);
            column += group.label().len() as u16 + 2;
        }
    }
}
