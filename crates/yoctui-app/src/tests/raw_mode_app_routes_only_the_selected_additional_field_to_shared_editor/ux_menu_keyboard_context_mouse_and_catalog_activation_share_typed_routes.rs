use super::*;

#[test]
fn ux_menu_keyboard_context_mouse_and_catalog_activation_share_typed_routes() {
    let mut app = yoctui_model::App::new(16, 4_096);
    assert_eq!(key_action(Input::F12), Some(Action::OpenApplicationMenu));
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
        Some(MenuInputResult::ActivateDisabled(
            "Load a Yocto workspace first".into()
        )),
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
        Some(Input::Alt('a'))
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

#[test]
fn menu_type_ahead_keeps_j_and_k_inside_command_names() {
    let mut app = yoctui_model::App::new(16, 4096);
    yoctui_model::update(&mut app, Action::OpenApplicationMenu);
    assert_eq!(
        menu_action(&app, Input::Char('k')),
        Some(MenuInputResult::Reduce(Box::new(Action::SelectMenuItem {
            delta: -1
        })))
    );
    for prefix in ["edit bbmas", "open ", "pro"] {
        app.menu.typed_prefix = prefix.into();
        for character in ['j', 'k'] {
            assert_eq!(
                menu_action(&app, Input::Char(character)),
                Some(MenuInputResult::Reduce(Box::new(Action::AppendMenuPrefix(
                    character
                ))))
            );
        }
    }
    app.menu.group_selection = yoctui_model::ApplicationMenuGroup::ALL
        .iter()
        .position(|g| *g == yoctui_model::ApplicationMenuGroup::Configuration)
        .unwrap();
    app.menu.typed_prefix.clear();
    for character in "edit bbmask".chars() {
        let Some(MenuInputResult::Reduce(action)) = menu_action(&app, Input::Char(character))
        else {
            panic!("type-ahead must reduce")
        };
        yoctui_model::update(&mut app, *action);
    }
    assert_eq!(app.selected_menu_item().unwrap().label, "Edit BBMASK");
    assert_eq!(app.menu.typed_prefix, "edit bbmask");
}
