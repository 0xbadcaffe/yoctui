use super::*;

#[test]
fn compatibility_dynamic_model_invalidation_closes_environment_dialog_but_keeps_local_one() {
    let mut app = App::new(10, 1_000);
    install_workspace_compatibility(
        &mut app,
        authority(
            1,
            vec![(
                CapabilityId::BitBakeBuild,
                CapabilityState::Available,
                Some("tinfoil.build"),
            )],
        ),
    )
    .unwrap();
    app.dialogs.push_front(Dialog::BuildOptions);
    let invalidated = invalidate_workspace_compatibility(&mut app);
    assert_eq!(invalidated.install, WorkspaceSnapshotInstall::Invalidated);
    assert!(invalidated.closed_dialog);

    app.dialogs.push_front(Dialog::QuitConfirmation);
    let local = invalidate_workspace_compatibility(&mut app);
    assert!(!local.closed_dialog);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::QuitConfirmation)
    ));
}
