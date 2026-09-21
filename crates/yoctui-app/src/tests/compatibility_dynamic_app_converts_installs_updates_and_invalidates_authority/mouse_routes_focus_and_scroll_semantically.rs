use super::*;

#[test]
fn mouse_routes_focus_and_scroll_semantically() {
    assert_eq!(
        mouse_action(
            MouseInput {
                kind: MouseKind::Down,
                column: 3,
                row: 4
            },
            140
        ),
        Some(Action::Focus(FocusTarget::Navigator))
    );
    assert_eq!(
        mouse_action(
            MouseInput {
                kind: MouseKind::Down,
                column: 130,
                row: 4
            },
            140
        ),
        Some(Action::Focus(FocusTarget::Inspector))
    );
    assert_eq!(
        mouse_action(
            MouseInput {
                kind: MouseKind::ScrollDown,
                column: 3,
                row: 4
            },
            80
        ),
        Some(Action::SelectNavigator { delta: 1 })
    );
}
