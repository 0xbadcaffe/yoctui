use super::*;

#[test]
fn devtool_editor_viewport_routes_complete_tree_navigation_and_vim_insert_mode() {
    let mut editor = yoctui_model::RecipeEditor {
        recipe: "demo".into(),
        root: "/workspace/demo".into(),
        files: (0..30)
            .map(|index| format!("file-{index}.rs").into())
            .collect(),
        file_inventory_truncated: false,
        selection: 12,
        focus: yoctui_model::RecipeEditorFocus::Files,
        language: yoctui_model::SourceLanguage::Rust,
        document: yoctui_model::TextAreaState::new("fn main() {}".into()),
        searching: false,
    };
    assert_eq!(
        recipe_editor_action(&editor, Input::PageDown),
        Some(Action::SelectRecipeEditorFile { delta: 10 })
    );
    assert_eq!(
        recipe_editor_action(&editor, Input::End),
        Some(Action::SelectRecipeEditorFile { delta: isize::MAX })
    );
    editor.focus = yoctui_model::RecipeEditorFocus::Document;
    assert_eq!(
        recipe_editor_action(&editor, Input::Char('i')),
        Some(Action::EditRecipeEditor(PopupEditorCommand::ToggleInsert))
    );
    editor.document.set_mode(yoctui_model::TextAreaMode::Insert);
    assert_eq!(
        recipe_editor_action(&editor, Input::Char('x')),
        Some(Action::EditRecipeEditor(PopupEditorCommand::Insert('x')))
    );
}
