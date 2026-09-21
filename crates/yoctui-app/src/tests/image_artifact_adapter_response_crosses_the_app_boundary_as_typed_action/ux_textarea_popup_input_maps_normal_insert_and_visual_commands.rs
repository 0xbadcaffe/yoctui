use super::*;

#[test]
fn ux_textarea_popup_input_maps_normal_insert_and_visual_commands() {
    assert_eq!(
        popup_editor_action(false, Input::Char('e')),
        Some(Action::EditActivePopup(PopupEditorCommand::SelectValue))
    );
    assert_eq!(
        popup_editor_action(false, Input::Char('j')),
        Some(Action::EditActivePopup(PopupEditorCommand::Down))
    );
    assert_eq!(
        popup_editor_action(false, Input::Char('x')),
        Some(Action::EditActivePopup(PopupEditorCommand::Delete))
    );
    assert_eq!(
        popup_editor_action(false, Input::Char('v')),
        Some(Action::EditActivePopup(PopupEditorCommand::ToggleVisual))
    );
    assert_eq!(
        popup_editor_action(false, Input::Char('u')),
        Some(Action::EditActivePopup(PopupEditorCommand::Undo))
    );
    assert_eq!(
        popup_editor_action(false, Input::PageDown),
        Some(Action::EditActivePopup(PopupEditorCommand::PageDown))
    );
    assert_eq!(
        popup_editor_action(true, Input::Char('k')),
        Some(Action::EditActivePopup(PopupEditorCommand::Insert('k')))
    );
    assert_eq!(
        popup_editor_action(true, Input::Home),
        Some(Action::EditActivePopup(PopupEditorCommand::Home))
    );
    assert_eq!(
        popup_editor_action(true, Input::CtrlV),
        Some(Action::EditActivePopup(PopupEditorCommand::Paste))
    );
    assert_eq!(
        popup_editor_action(true, Input::Enter),
        Some(Action::EditActivePopup(PopupEditorCommand::Newline))
    );
}
