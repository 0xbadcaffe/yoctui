use super::*;

#[test]
fn maintenance_release_archive_workspace_maps_text_toggles_and_cancel() {
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Release, 3, Input::Char('a')),
        Some(Action::Maintenance(MaintenanceAction::OpenGitArchiveForm))
    );
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Services, 3, Input::Char('a')),
        None
    );
    let mut editor = yoctui_model::PopupEditor::new(
        "data_dir = \"\"\ncreate = true\npush_remote = \"\"\n".into(),
    );
    editor.editing = true;
    assert!(matches!(
        maintenance_dialog_action(
            &MaintenanceDialog::GitArchiveToml {
                editor: editor.clone(),
                validation_error: None,
            },
            Input::Char('/'),
        ),
        Some(Action::EditActivePopup(PopupEditorCommand::Insert('/')))
    ));
    assert!(matches!(
        maintenance_dialog_action(
            &MaintenanceDialog::GitArchiveToml {
                editor: editor.clone(),
                validation_error: None,
            },
            Input::Enter,
        ),
        Some(Action::Maintenance(MaintenanceAction::ConfirmGitArchiveToml(document)))
            if document == editor.text
    ));
    editor.editing = false;
    assert_eq!(
        maintenance_dialog_action(
            &MaintenanceDialog::GitArchiveToml {
                editor,
                validation_error: None,
            },
            Input::Esc,
        ),
        Some(Action::Maintenance(MaintenanceAction::CancelDialog))
    );
}
