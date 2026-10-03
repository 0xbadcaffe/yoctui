use super::*;

#[test]
fn tasks_navigator_grouped_mouse_matches_visible_model_rows() {
    for (width, height, compact) in [
        (160, 50, false),
        (160, 48, true),
        (150, 50, false),
        (200, 60, false),
        (120, 40, false),
        (80, 24, false),
    ] {
        let mut app = App::new(10, 1024);
        app.screen = Screen::Tasks;
        app.focus = FocusTarget::Navigator;
        app.preferences.mouse_enabled = true;
        if compact {
            app.preferences.density = yoctui_model::UiDensity::Compact;
        }
        let shell = workbench_shell(&app, width, height).unwrap();
        let probe = MouseInput {
            kind: MouseKind::Down,
            column: shell.x + 1,
            row: shell.y + 2,
        };
        let region = workbench_mouse_region(probe, &app, shell).unwrap();
        assert_eq!(region.target, FocusTarget::Navigator);
        assert_ne!(region.area.width, 26);
        let rows = usize::from(region.area.height.saturating_sub(2));
        let start = app.navigator_viewport_start(rows);
        for row in 0..rows {
            let visual = start + row;
            let mouse = MouseInput {
                row: region.area.y + 1 + row as u16,
                ..probe
            };
            let action = mouse_action_for_app(mouse, &app, width, height);
            if let Some(group) = app.navigator_group_at_visual_row(visual) {
                assert_eq!(action, Some(Action::ToggleNavigatorGroup { group }));
            } else if let Some(index) = app.navigator_selection_at_visual_row(visual) {
                let expected = if app.navigator_selection == index {
                    Action::ActivateNavigator
                } else {
                    Action::SelectNavigatorAt { index }
                };
                assert_eq!(action, Some(expected), "{width}x{height}, row{row}");
                if index != app.navigator_selection {
                    let mut selected = app.clone();
                    yoctui_model::update(&mut selected, action.unwrap());
                    assert_eq!(selected.navigator_selection, index);
                    assert_eq!(selected.focus, FocusTarget::Navigator);
                    assert_eq!(selected.screen, Screen::Tasks);
                }
            }
        }
    }
}

#[test]
fn tasks_navigator_literal_rows_keep_exact_visible_destinations() {
    let mut app = App::new(10, 1024);
    for (row, expected) in [
        (0, Screen::Layers),
        (1, Screen::Layers),
        (2, Screen::Recipes),
        (3, Screen::Recipes),
        (4, Screen::Images),
        (5, Screen::Images),
        (6, Screen::Tasks),
        (7, Screen::Tasks),
        (8, Screen::Testing),
        (9, Screen::Qa),
        (10, Screen::Devtool),
        (11, Screen::Images),
        (12, Screen::Sdk),
        (13, Screen::Security),
        (14, Screen::Maintenance),
    ] {
        app.navigator_selection = literal_navigator_selection_at_row(&app, row).unwrap();
        assert_eq!(app.navigator_screen(), expected, "literal row{row}");
    }
    assert_eq!(literal_navigator_selection_at_row(&app, 15), None);
}
