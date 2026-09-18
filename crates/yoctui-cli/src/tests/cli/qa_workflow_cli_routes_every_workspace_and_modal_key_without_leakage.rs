use super::*;

#[test]
fn qa_workflow_cli_routes_every_workspace_and_modal_key_without_leakage() {
    use yoctui_app::Input;
    let keys = [
        Input::Tab,
        Input::Up,
        Input::Down,
        Input::Char('s'),
        Input::Char('/'),
        Input::Char('f'),
        Input::Char('r'),
        Input::Char('I'),
        Input::Char('R'),
        Input::Enter,
        Input::Char('o'),
        Input::Char('e'),
        Input::Char('l'),
        Input::Char('c'),
    ];
    for key in keys {
        assert!(
            qa_workspace_action(yoctui_model::QaView::RecipeKernel, false, false, key).is_some(),
            "unrouted QA workspace key: {key:?}"
        );
    }
    let mut editor = yoctui_model::PopupEditor::new("root = \"\"\n".into());
    editor.select_toml_value("root").unwrap();
    editor.editing = true;
    let dialog = yoctui_model::QaDialog::Import {
        editor,
        validation_error: None,
    };
    assert!(qa_dialog_action(&dialog, Input::Char('x')).is_some());
    assert!(qa_dialog_action(&dialog, Input::Backspace).is_some());
    assert!(qa_dialog_action(&dialog, Input::Enter).is_some());
    assert!(qa_dialog_action(&dialog, Input::Esc).is_some());
    assert!(qa_dialog_action(&dialog, Input::Tab).is_none());
}
