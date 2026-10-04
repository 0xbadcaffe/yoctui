use super::*;

fn editor(focus: yoctui_model::RecipeEditorFocus) -> yoctui_model::RecipeEditor {
    yoctui_model::RecipeEditor {
        recipe: "busybox".into(),
        root: "/workspace/busybox".into(),
        files: vec!["src/main.c".into()],
        file_inventory_truncated: false,
        context: Default::default(),
        selection: 0,
        focus,
        language: yoctui_model::SourceLanguage::C,
        document: yoctui_model::TextAreaState::new("int main(void) {}".into()),
        searching: false,
        pending_search_position: None,
    }
}

#[test]
fn devtool_editor_search_routes_file_workspace_and_global_scopes() {
    for focus in [
        yoctui_model::RecipeEditorFocus::Files,
        yoctui_model::RecipeEditorFocus::Document,
    ] {
        let editor = editor(focus);
        assert_eq!(
            recipe_editor_action(&editor, Input::CtrlF),
            Some(Action::BeginRecipeEditorSearch)
        );
        assert_eq!(
            recipe_editor_action(&editor, Input::CtrlShiftF),
            Some(Action::OpenRecipeEditorWorkspaceSearch)
        );
        assert_eq!(
            recipe_editor_action(&editor, Input::Char('/')),
            Some(Action::OpenGlobalSearch)
        );
    }
}

#[test]
fn devtool_editor_search_keeps_slash_editable_in_insert_mode() {
    let mut editor = editor(yoctui_model::RecipeEditorFocus::Document);
    editor.document.set_mode(yoctui_model::TextAreaMode::Insert);
    assert_eq!(
        recipe_editor_action(&editor, Input::Char('/')),
        Some(Action::EditRecipeEditor(
            yoctui_model::PopupEditorCommand::Insert('/')
        ))
    );
    assert_eq!(
        recipe_editor_action(&editor, Input::CtrlShiftF),
        Some(Action::OpenRecipeEditorWorkspaceSearch)
    );
}

#[test]
fn modifier_editor_commands_work_in_every_mode_without_inserting_letters() {
    for focus in [
        yoctui_model::RecipeEditorFocus::Files,
        yoctui_model::RecipeEditorFocus::Document,
    ] {
        for mode in [
            yoctui_model::TextAreaMode::Normal,
            yoctui_model::TextAreaMode::Insert,
            yoctui_model::TextAreaMode::Visual,
        ] {
            let mut editor = editor(focus);
            editor.document.set_mode(mode);
            for searching in [false, true] {
                editor.searching = searching;
                assert_eq!(
                    recipe_editor_action(&editor, Input::Alt('f')),
                    Some(Action::OpenRecipeEditorWorkspaceSearch)
                );
                assert_eq!(
                    recipe_editor_action(&editor, Input::Alt('g')),
                    Some(Action::OpenRecipeEditorGitUi)
                );
            }
        }
    }
}

#[test]
fn modifier_workspace_routes_match_menus_and_preserve_search_text() {
    for route in [recipes_workspace_action, devtool_workspace_action] {
        assert_eq!(
            route(false, Input::Alt('w')),
            Some(Action::BeginSelectedRecipeDevtoolModify)
        );
        assert_eq!(
            route(false, Input::Alt('g')),
            Some(Action::BeginSelectedRecipeDevtoolGitUi)
        );
        assert_eq!(
            route(true, Input::Char('G')),
            Some(Action::AppendMetadataQuery('G'))
        );
        assert_eq!(route(true, Input::Alt('g')), None);
    }
    assert_eq!(
        context_menu_activation_input("recipes.devtool_gitui"),
        Some(Input::Alt('g'))
    );
    assert_eq!(
        context_menu_activation_input("devtool.gitui"),
        Some(Input::Alt('g'))
    );
}

#[test]
fn modifier_command_aliases_preserve_existing_workspace_actions() {
    let routes: &[fn(bool, Input) -> Option<Action>] = &[
        recipes_workspace_action,
        devtool_workspace_action,
        config_workspace_action,
        layer_tree_action,
        package_workspace_action,
        sdk_workspace_action,
        dependency_workspace_action,
        images_workspace_action,
    ];
    for route in routes {
        for letter in 'A'..='Z' {
            // G is the legacy Vim End alias; Alt+g intentionally opens GitUI.
            if letter == 'G' {
                continue;
            }
            if let Some(expected) = route(false, Input::Char(letter)) {
                assert_eq!(
                    route(false, Input::Alt(letter.to_ascii_lowercase())),
                    Some(expected),
                    "{letter}"
                );
            }
        }
    }
}
