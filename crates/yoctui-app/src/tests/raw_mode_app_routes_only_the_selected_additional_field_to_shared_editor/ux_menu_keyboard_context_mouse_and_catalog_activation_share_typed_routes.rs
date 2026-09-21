use super::*;

#[test]
fn ux_menu_keyboard_context_mouse_and_catalog_activation_share_typed_routes() {
    let mut app = yoctui_model::App::new(16, 4_096);
    assert_eq!(key_action(Input::F10), Some(Action::OpenApplicationMenu));
    let _ = yoctui_model::update(&mut app, Action::OpenApplicationMenu);
    assert_eq!(
        menu_action(&app, Input::Right),
        Some(MenuInputResult::Reduce(Box::new(Action::SelectMenuGroup {
            delta: 1
        })))
    );
    let _ = yoctui_model::update(&mut app, Action::SelectMenuGroup { delta: 1 });
    assert_eq!(app.menu.group(), yoctui_model::ApplicationMenuGroup::Build);
    assert_eq!(
        menu_action(&app, Input::Enter),
        None,
        "build stays disabled without a workspace"
    );

    let _ = yoctui_model::update(&mut app, Action::CloseMenu);
    app.screen = Screen::Recipes;
    assert_eq!(
        mouse_action_for_app(
            MouseInput {
                kind: MouseKind::ContextDown,
                column: 40,
                row: 8,
            },
            &app,
            160,
            48,
        ),
        Some(Action::OpenContextMenu)
    );
    assert_eq!(key_action(Input::Char('a')), Some(Action::OpenContextMenu));
    assert_eq!(
        context_menu_activation_input("recipes.dependencies"),
        Some(Input::Char('A'))
    );
    assert!(
        yoctui_model::operator_action_catalog()
            .into_iter()
            .filter_map(|definition| match definition.target {
                yoctui_model::OperatorActionTarget::Workspace { legacy_id, .. } => {
                    Some(legacy_id)
                }
                yoctui_model::OperatorActionTarget::Command(_) => None,
            })
            .all(|id| context_menu_activation_input(id).is_some()),
        "every contextual catalog entry must retain a typed legacy route"
    );
}
