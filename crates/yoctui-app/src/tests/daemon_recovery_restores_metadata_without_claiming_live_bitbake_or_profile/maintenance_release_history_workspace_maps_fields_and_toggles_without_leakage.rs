use super::*;

#[test]
fn maintenance_release_history_workspace_maps_fields_and_toggles_without_leakage() {
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Release, 3, Input::Char('h')),
        Some(Action::Maintenance(MaintenanceAction::OpenBuildHistoryForm))
    );
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Sstate, 3, Input::Char('h')),
        None
    );
    let mut editor =
        yoctui_model::PopupEditor::new("from_revision = \"\"\nreport_version = false\n".into());
    editor.editing = true;
    assert!(matches!(
        maintenance_dialog_action(
            &MaintenanceDialog::BuildHistoryToml {
                editor: editor.clone(),
                validation_error: None
            },
            Input::Char('H'),
        ),
        Some(Action::EditActivePopup(PopupEditorCommand::Insert('H')))
    ));
    assert!(matches!(
        maintenance_dialog_action(
            &MaintenanceDialog::BuildHistoryToml {
                editor: editor.clone(),
                validation_error: None
            },
            Input::Enter,
        ),
        Some(Action::Maintenance(
            MaintenanceAction::ConfirmBuildHistoryToml(_)
        ))
    ));
    editor.editing = false;
    assert_eq!(
        maintenance_dialog_action(
            &MaintenanceDialog::BuildHistoryToml {
                editor,
                validation_error: None
            },
            Input::Esc,
        ),
        Some(Action::Maintenance(MaintenanceAction::CancelDialog))
    );
}
