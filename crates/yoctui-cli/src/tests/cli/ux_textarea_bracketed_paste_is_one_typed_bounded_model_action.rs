use super::*;

#[test]
fn ux_textarea_bracketed_paste_is_one_typed_bounded_model_action() {
    let mut app = yoctui_model::App::new(10, 1_000);
    let mut editor = yoctui_model::PopupEditor::new("root = \"\"\n".into());
    editor.set_mode(yoctui_model::TextAreaMode::Insert);
    app.dialogs
        .push_front(Dialog::BuildEnvironmentEditor(editor));
    assert!(active_popup_accepts_paste(&app));
    let _ = update(
        &mut app,
        Action::EditActivePopup(yoctui_model::PopupEditorCommand::PasteText {
            text: "猫\nvalue = café".into(),
            source: yoctui_model::TextAreaPasteSource::BracketedPaste,
        }),
    );
    let Some(Dialog::BuildEnvironmentEditor(editor)) = app.active_dialog() else {
        panic!("editor dialog missing")
    };
    assert!(editor.text.ends_with("猫\nvalue = café"));
    assert_eq!(editor.history_lengths(), (1, 0));
}
