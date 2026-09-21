use super::*;

#[test]
fn ux_responsive_mouse_regions_keyboard_scroll_and_minimum_are_exact() {
    let mut app = yoctui_model::App::new(16, 4096);
    app.screen = Screen::Recipes;

    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 100,
                row: 10,
            },
            &app,
            160,
            48,
        ),
        Some(Action::Focus(FocusTarget::Workspace))
    );
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::ScrollUp,
                column: 30,
                row: 10,
            },
            &app,
            160,
            48,
        ),
        recipes_workspace_action(false, Input::Up)
    );
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 8,
                row: 2,
            },
            &app,
            90,
            30,
        ),
        Some(Action::Focus(FocusTarget::Navigator))
    );
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 22,
                row: 2,
            },
            &app,
            90,
            30,
        ),
        Some(Action::Focus(FocusTarget::Workspace))
    );
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 32,
                row: 2,
            },
            &app,
            90,
            30,
        ),
        None
    );
    for inert in [
        MouseInput {
            kind: MouseKind::Down,
            column: 30,
            row: 0,
        },
        MouseInput {
            kind: MouseKind::Down,
            column: 30,
            row: 29,
        },
    ] {
        assert_eq!(mouse_action_for_app(inert, &app, 90, 30), None);
    }
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::Down,
                column: 5,
                row: 5,
            },
            &app,
            79,
            23,
        ),
        None,
        "the below-minimum resize screen is inert"
    );
}
