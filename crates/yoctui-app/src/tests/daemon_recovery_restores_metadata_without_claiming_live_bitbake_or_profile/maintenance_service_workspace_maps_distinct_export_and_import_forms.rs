use super::*;

#[test]
fn maintenance_service_workspace_maps_distinct_export_and_import_forms() {
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Services, 1, Input::Char('e')),
        Some(Action::Maintenance(MaintenanceAction::OpenPrServiceForm(
            yoctui_model::PrServiceOperation::Export,
        )))
    );
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Services, 1, Input::Char('m')),
        Some(Action::Maintenance(MaintenanceAction::OpenPrServiceForm(
            yoctui_model::PrServiceOperation::Import,
        )))
    );
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Sstate, 2, Input::Char('e')),
        None
    );
    let mut editor = yoctui_model::PopupEditor::new("file = \"\"\n".into());
    editor.select_range(8, 8);
    editor.editing = true;
    assert!(matches!(
        maintenance_dialog_action(
            &MaintenanceDialog::PrServiceToml {
                operation: yoctui_model::PrServiceOperation::Import,
                editor: editor.clone(),
                validation_error: None,
            },
            Input::Char('/'),
        ),
        Some(Action::EditActivePopup(PopupEditorCommand::Insert('/')))
    ));
    assert!(matches!(
        maintenance_dialog_action(
            &MaintenanceDialog::PrServiceToml {
                operation: yoctui_model::PrServiceOperation::Import,
                editor: editor.clone(),
                validation_error: None,
            },
            Input::Enter,
        ),
        Some(Action::Maintenance(MaintenanceAction::ConfirmPrServiceToml {
            operation: yoctui_model::PrServiceOperation::Import,
            document,
        })) if document == editor.text
    ));
}
