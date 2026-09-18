use super::*;

#[test]
fn recipe_bitbake_action_terminal_keys_decode_to_typed_routes() {
    let force = input_from_key(KeyEvent::new(KeyCode::Char('f'), KeyModifiers::NONE)).unwrap();
    let devshell = input_from_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE)).unwrap();
    assert_eq!(
        yoctui_app::recipes_workspace_action(false, force),
        Some(Action::BeginSelectedRecipeForceTask)
    );
    assert_eq!(
        yoctui_app::recipes_workspace_action(false, devshell),
        Some(Action::BeginSelectedRecipeDevshell)
    );
}
