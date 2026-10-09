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
        context: Default::default(),
        selection: 12,
        focus: yoctui_model::RecipeEditorFocus::Files,
        language: yoctui_model::SourceLanguage::Rust,
        document: yoctui_model::TextAreaState::new("fn main() {}".into()),
        searching: false,
        pending_search_position: None,
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
    editor.context = yoctui_model::SourceEditorContext::DeviceTree;
    assert_eq!(
        recipe_editor_action(&editor, Input::PageDown),
        Some(Action::EditRecipeEditor(PopupEditorCommand::PageDown))
    );
    assert_eq!(
        recipe_editor_action(&editor, Input::Alt('f')),
        Some(Action::OpenRecipeEditorWorkspaceSearch)
    );
    for (key, line) in [(Input::Home, 0), (Input::End, usize::MAX)] {
        assert_eq!(
            recipe_editor_action(&editor, key),
            Some(Action::EditRecipeEditor(
                PopupEditorCommand::SelectPosition {
                    line,
                    column: 0,
                    extend: false
                }
            ))
        );
    }
    let mut app = yoctui_model::App::new(32, 8192);
    app.focus = FocusTarget::Dialog;
    app.dialogs
        .push_back(yoctui_model::Dialog::RecipeEditor(editor.clone()));
    for (kind, command) in [
        (MouseKind::ScrollDown, PopupEditorCommand::Down),
        (MouseKind::ScrollUp, PopupEditorCommand::Up),
    ] {
        assert_eq!(
            mouse_action_for_app(
                MouseInput {
                    kind,
                    column: 50,
                    row: 12
                },
                &app,
                160,
                50
            ),
            Some(Action::EditRecipeEditor(command))
        );
    }
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
