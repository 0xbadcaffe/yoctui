use super::*;

#[test]
fn maintenance_sstate_workspace_maps_only_typed_form_input() {
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Sstate, 2, Input::Char('c')),
        Some(Action::Maintenance(MaintenanceAction::OpenReadinessForm))
    );
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Sstate, 2, Input::Char('d')),
        Some(Action::Maintenance(MaintenanceAction::OpenCleanupForm))
    );
    assert_eq!(
        maintenance_workspace_action(MaintenanceView::Release, 4, Input::Char('c')),
        None
    );

    let mut readiness = yoctui_model::PopupEditor::new(
        "targets = \"\"\nmode = \"isolated_tmpdir\"\ntimeout = 3600\n".into(),
    );
    readiness.select_range(11, 11);
    readiness.editing = true;
    assert!(matches!(
        maintenance_dialog_action(
            &MaintenanceDialog::ReadinessToml {
                editor: readiness.clone(),
                validation_error: None,
            },
            Input::Char('c'),
        ),
        Some(Action::EditActivePopup(PopupEditorCommand::Insert('c')))
    ));
    assert!(matches!(
        maintenance_dialog_action(
            &MaintenanceDialog::ReadinessToml {
                editor: readiness.clone(),
                validation_error: None,
            },
            Input::Enter,
        ),
        Some(Action::Maintenance(
            MaintenanceAction::ConfirmReadinessToml(document)
        )) if document == readiness.text
    ));

    let cleanup = yoctui_model::PopupEditor::new(
        "duplicates = true\norphans = false\nunreferenced_by_stamps = false\njobs = 1\n".into(),
    );
    assert!(matches!(
        maintenance_dialog_action(
            &MaintenanceDialog::CleanupToml {
                editor: cleanup.clone(),
                validation_error: None,
            },
            Input::Char('e'),
        ),
        Some(Action::EditActivePopup(PopupEditorCommand::SelectValue))
    ));
    assert!(matches!(
        maintenance_dialog_action(
            &MaintenanceDialog::CleanupToml {
                editor: cleanup.clone(),
                validation_error: None,
            },
            Input::Enter,
        ),
        Some(Action::Maintenance(MaintenanceAction::ConfirmCleanupToml(document)))
            if document == cleanup.text
    ));
    assert_eq!(
        maintenance_dialog_action(
            &MaintenanceDialog::CleanupToml {
                editor: cleanup,
                validation_error: None,
            },
            Input::Esc,
        ),
        Some(Action::Maintenance(MaintenanceAction::CancelDialog))
    );
}
