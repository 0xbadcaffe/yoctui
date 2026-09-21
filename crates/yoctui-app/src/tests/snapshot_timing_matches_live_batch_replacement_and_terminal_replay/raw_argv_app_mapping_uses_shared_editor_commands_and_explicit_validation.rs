use super::*;

#[test]
fn raw_argv_app_mapping_uses_shared_editor_commands_and_explicit_validation() {
    assert_eq!(
        raw_argv_editor_action(false, Input::Char('i')),
        Some(RawArgvEditorAction::Edit(
            yoctui_model::PopupEditorCommand::ToggleInsert
        ))
    );
    assert_eq!(
        raw_argv_editor_action(true, Input::Char('x')),
        Some(RawArgvEditorAction::Edit(
            yoctui_model::PopupEditorCommand::Insert('x')
        ))
    );
    assert_eq!(
        raw_argv_editor_action(true, Input::Esc),
        Some(RawArgvEditorAction::Edit(
            yoctui_model::PopupEditorCommand::ToggleInsert
        ))
    );
    assert_eq!(
        raw_argv_editor_action(false, Input::Enter),
        Some(RawArgvEditorAction::Validate)
    );
    assert_eq!(
        raw_argv_editor_action(false, Input::Char('x')),
        Some(RawArgvEditorAction::Edit(
            yoctui_model::PopupEditorCommand::Delete
        ))
    );
}
