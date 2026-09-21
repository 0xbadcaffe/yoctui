use super::*;

#[test]
fn maintenance_workflow_dialog_mapping_traps_typed_input() {
    let preview = maintenance_preview();
    assert_eq!(
        maintenance_dialog_action(&MaintenanceDialog::Confirm(preview.clone()), Input::Enter),
        Some(Action::Maintenance(MaintenanceAction::ConfirmOperation(
            preview.clone()
        )))
    );
    assert_eq!(
        maintenance_dialog_action(
            &MaintenanceDialog::CleanupPhrase {
                preview: preview.clone(),
                input: "DELETE".into()
            },
            Input::Char(' ')
        ),
        Some(Action::Maintenance(
            MaintenanceAction::UpdateCleanupPhrase {
                preview: preview.clone(),
                input: "DELETE ".into()
            }
        ))
    );
    assert_eq!(
        maintenance_dialog_action(&MaintenanceDialog::ConfirmNetworkPush(preview), Input::Esc),
        Some(Action::Maintenance(MaintenanceAction::CancelDialog))
    );
}
