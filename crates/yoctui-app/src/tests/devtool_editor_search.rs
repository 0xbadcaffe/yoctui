use super::*;

fn editor(focus: yoctui_model::RecipeEditorFocus) -> yoctui_model::RecipeEditor {
    yoctui_model::RecipeEditor {
        recipe: "busybox".into(),
        root: "/workspace/busybox".into(),
        files: vec!["src/main.c".into()],
        file_inventory_truncated: false,
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
