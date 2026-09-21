use super::*;

#[test]
fn maintenance_workflow_requires_exact_cleanup_phrase() {
    let mut state = ready_state();
    let cleanup = SstateCleanupPreview::new(
        SstateCleanupRequest::new(
            "/cache".into(),
            vec![],
            vec![SstateCleanupMode::Duplicates],
            4,
        )
        .unwrap(),
        vec![identity("/cache/a"), identity("/cache/b")],
    )
    .unwrap();
    assert_eq!(cleanup.required_phrase(), "DELETE 2 FROM /cache");
    assert!(
        SstateCleanupPreview::new(cleanup.request.clone(), vec![identity("/outside/a")]).is_err()
    );

    let preview = MaintenanceOperationPreview::new(
        9,
        1,
        MaintenanceOperation::SstateCleanup(cleanup),
        vec!["0: /tools/sstate".into()],
        vec![],
    )
    .unwrap();
    let transition = update_maintenance(
        &mut state,
        MaintenanceAction::BeginOperation(preview.clone()),
    );
    assert!(matches!(
        transition.dialog,
        MaintenanceDialogUpdate::Open(dialog)
            if matches!(*dialog, MaintenanceDialog::CleanupPhrase { .. })
    ));
    assert!(
        state
            .pending
            .as_ref()
            .is_some_and(|value| value == &preview)
    );
    assert_eq!(
        update_maintenance(
            &mut state,
            MaintenanceAction::ConfirmCleanupPhrase {
                preview: preview.clone(),
                input: "DELETE 1 FROM /cache".into(),
            },
        ),
        MaintenanceTransition::none()
    );
    assert!(matches!(
        update_maintenance(
            &mut state,
            MaintenanceAction::ConfirmCleanupPhrase {
                preview,
                input: "DELETE 2 FROM /cache".into(),
            },
        )
        .dialog,
        MaintenanceDialogUpdate::Open(dialog)
            if matches!(*dialog, MaintenanceDialog::Confirm(_))
    ));
}
