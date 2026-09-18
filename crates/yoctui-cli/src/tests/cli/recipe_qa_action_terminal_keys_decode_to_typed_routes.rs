use super::*;

#[test]
fn recipe_qa_action_terminal_keys_decode_to_typed_routes() {
    for (key, expected) in [
        ('V', Action::BeginSelectedRecipeCveCheck),
        ('X', Action::BeginSelectedRecipeSpdx),
    ] {
        let input = input_from_key(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE)).unwrap();
        assert_eq!(
            yoctui_app::recipes_workspace_action(false, input),
            Some(expected)
        );
    }
}
