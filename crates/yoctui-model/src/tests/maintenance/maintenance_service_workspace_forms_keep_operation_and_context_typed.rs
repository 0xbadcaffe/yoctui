use super::*;

#[test]
fn maintenance_service_workspace_forms_keep_operation_and_context_typed() {
    for operation in [PrServiceOperation::Export, PrServiceOperation::Import] {
        let mut state = ready_state();
        state.view = MaintenanceView::Services;
        let transition =
            update_maintenance(&mut state, MaintenanceAction::OpenPrServiceForm(operation));
        let MaintenanceDialogUpdate::Open(dialog) = transition.dialog else {
            panic!("PR service form did not open");
        };
        let MaintenanceDialog::PrServiceToml {
            operation: actual,
            editor,
            ..
        } = *dialog
        else {
            panic!("wrong PR service dialog");
        };
        assert_eq!(actual, operation);
        assert_eq!(editor.selected_text(), Some(""));
        assert!(
            editor
                .text
                .contains("# Build directory (read-only): /build")
        );
        assert!(
            editor
                .text
                .contains("# Endpoint (read-only): localhost:8585")
        );
        let invalid = update_maintenance(
            &mut state,
            MaintenanceAction::ConfirmPrServiceToml {
                operation,
                document: "file = \"/evidence/pr.txt\"\n".into(),
            },
        );
        assert!(matches!(
            invalid.dialog,
            MaintenanceDialogUpdate::Open(dialog)
                if matches!(*dialog, MaintenanceDialog::PrServiceToml { validation_error: Some(_), .. })
        ));
        let file = match operation {
            PrServiceOperation::Export => "/evidence/pr.conf",
            PrServiceOperation::Import => "/evidence/pr.inc",
        };
        let valid = update_maintenance(
            &mut state,
            MaintenanceAction::ConfirmPrServiceToml {
                operation,
                document: format!("file = \"{file}\"\n"),
            },
        );
        assert!(matches!(
            valid.effect,
            Some(MaintenanceEffect::PreviewPrService {
                capability_request: 1,
                request: PrServiceRequest { operation: actual, .. },
            }) if actual == operation
        ));
    }
}
