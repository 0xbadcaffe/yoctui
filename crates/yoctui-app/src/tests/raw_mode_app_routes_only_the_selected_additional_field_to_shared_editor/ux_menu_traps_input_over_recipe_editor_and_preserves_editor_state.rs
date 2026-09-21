use super::*;

#[test]
fn ux_menu_traps_input_over_recipe_editor_and_preserves_editor_state() {
    let mut app = yoctui_model::App::new(16, 4_096);
    app.screen = Screen::Recipes;
    app.workspace.build_dir = Some("/workspace/build".into());
    let root = std::path::PathBuf::from("/workspace/bash");
    let _ = yoctui_model::update(
        &mut app,
        Action::OpenRecipeEditor {
            recipe: "bash".into(),
            root,
            files: vec!["bash.bb".into()],
        },
    );
    let _ = yoctui_model::update(
        &mut app,
        Action::LoadRecipeEditorContent("SUMMARY = \"bash\"\n".into()),
    );
    let _ = yoctui_model::update(&mut app, Action::ToggleRecipeEditorEditing);
    let _ = yoctui_model::update(&mut app, Action::AppendRecipeEditor('X'));
    let editor_before = app.active_dialog().cloned();

    let _ = yoctui_model::update(&mut app, Action::OpenApplicationMenu);
    let _ = yoctui_model::update(&mut app, Action::SelectMenuGroup { delta: 1 });
    let _ = yoctui_model::update(&mut app, Action::SelectMenuItem { delta: 2 });
    assert_eq!(app.focus, FocusTarget::Dialog);
    assert_eq!(app.menu.group(), yoctui_model::ApplicationMenuGroup::Build);
    assert_eq!(
        app.selected_menu_item()
            .and_then(|item| item.disabled_reason),
        Some("No active build is available to cancel.".into())
    );
    assert_eq!(menu_action(&app, Input::Enter), None);
    assert_eq!(
        menu_action(&app, Input::Char('c')),
        Some(MenuInputResult::Reduce(Box::new(Action::AppendMenuPrefix(
            'c'
        ))))
    );
    assert_eq!(app.active_dialog(), editor_before.as_ref());

    let _ = yoctui_model::update(&mut app, Action::CloseMenu);
    assert!(matches!(
        app.active_dialog(),
        Some(yoctui_model::Dialog::RecipeEditor(_))
    ));
}
