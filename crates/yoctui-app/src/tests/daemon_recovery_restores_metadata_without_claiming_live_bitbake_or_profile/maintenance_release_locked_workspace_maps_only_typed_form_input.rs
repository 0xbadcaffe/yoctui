use super::*;

#[test]
fn maintenance_release_locked_workspace_maps_only_typed_form_input() {
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Release, 3, Input::Char('l')),
        Some(Action::Maintenance(MaintenanceAction::OpenLockedCacheForm))
    );
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Services, 3, Input::Char('l')),
        None
    );
    let mut editor = yoctui_model::PopupEditor::new(
        "locked_signatures = \"\"\ninput_cache = \"\"\noutput_cache = \"\"\nfilter = \"\"\n".into(),
    );
    editor.select_range(21, 21);
    editor.editing = true;
    assert!(matches!(
        maintenance_dialog_action(
            &MaintenanceDialog::LockedCacheToml {
                editor: editor.clone(),
                validation_error: None,
            },
            Input::Char('/'),
        ),
        Some(Action::EditActivePopup(PopupEditorCommand::Insert('/')))
    ));
    assert!(matches!(
        maintenance_dialog_action(
            &MaintenanceDialog::LockedCacheToml {
                editor: editor.clone(),
                validation_error: None,
            },
            Input::Enter,
        ),
        Some(Action::Maintenance(MaintenanceAction::ConfirmLockedCacheToml(document)))
            if document == editor.text
    ));
    editor.editing = false;
    assert_eq!(
        maintenance_dialog_action(
            &MaintenanceDialog::LockedCacheToml {
                editor,
                validation_error: None,
            },
            Input::Esc,
        ),
        Some(Action::Maintenance(MaintenanceAction::CancelDialog))
    );
}
