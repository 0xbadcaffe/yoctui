use super::*;

#[test]
fn devwork_editor_routes_confirmation_and_workspace_editor_build_keys() {
    let mut editor = yoctui_model::RecipeEditor {
        recipe: "busybox".into(),
        root: "/workspace/busybox".into(),
        files: vec!["main.c".into()],
        file_inventory_truncated: false,
        selection: 0,
        focus: yoctui_model::RecipeEditorFocus::Files,
        language: yoctui_model::SourceLanguage::C,
        document: yoctui_model::TextAreaState::new("int main() {}".into()),
        searching: false,
        pending_search_position: None,
    };
    assert_eq!(
        devtool_modify_confirmation_action(Input::Enter),
        Some(Action::ConfirmDevtoolModify)
    );
    assert_eq!(
        devtool_modify_confirmation_action(Input::Esc),
        Some(Action::CancelDevtoolModify)
    );
    assert_eq!(devtool_modify_confirmation_action(Input::Char('b')), None);
    assert_eq!(
        recipe_editor_action(&editor, Input::CtrlB),
        Some(Action::BeginRecipeEditorBuild)
    );
    assert_eq!(
        recipe_editor_action(&editor, Input::Enter),
        Some(Action::FocusRecipeEditor(
            yoctui_model::RecipeEditorFocus::Document
        ))
    );
    editor.focus = yoctui_model::RecipeEditorFocus::Document;
    editor.document.set_mode(yoctui_model::TextAreaMode::Insert);
    assert_eq!(
        recipe_editor_action(&editor, Input::Enter),
        Some(Action::EditRecipeEditor(PopupEditorCommand::Newline))
    );
}
