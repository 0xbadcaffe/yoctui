use super::*;

#[test]
fn dependency_workspace_terminal_keys_decode_to_typed_routes() {
    for (code, expected) in [
        (KeyCode::Up, Action::SelectDependencyGraphNode { delta: -1 }),
        (
            KeyCode::Down,
            Action::SelectDependencyGraphNode { delta: 1 },
        ),
        (KeyCode::Enter, Action::OpenSelectedDependencyRecipe),
        (KeyCode::Char('o'), Action::OpenSelectedDependencyProvider),
        (KeyCode::Char('L'), Action::OpenSelectedDependencyTaskLog),
        (KeyCode::Char('r'), Action::RefreshDependencyGraph),
    ] {
        let input = input_from_key(KeyEvent::new(code, KeyModifiers::NONE)).unwrap();
        assert_eq!(dependency_workspace_action(false, input), Some(expected));
    }
}
