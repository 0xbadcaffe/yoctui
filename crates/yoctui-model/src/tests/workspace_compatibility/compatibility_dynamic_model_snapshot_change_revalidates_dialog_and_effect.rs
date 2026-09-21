use super::*;

#[test]
fn compatibility_dynamic_model_snapshot_change_revalidates_dialog_and_effect() {
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Packages;
    app.navigator_selection = 3;
    app.dialogs.push_front(Dialog::BuildOptions);
    app.focus = crate::FocusTarget::Dialog;
    app.focus_return = Some(crate::FocusTarget::Inspector);
    let available = authority(
        1,
        vec![(
            CapabilityId::BitBakeBuild,
            CapabilityState::Available,
            Some("tinfoil.build"),
        )],
    );
    let retained = install_workspace_compatibility(&mut app, available).unwrap();
    assert!(!retained.closed_dialog);
    assert!(matches!(app.active_dialog(), Some(Dialog::BuildOptions)));
    let start = Effect::Start(BuildRequest {
        targets: vec!["core-image-minimal".into()],
        task: None,
        force: false,
    });
    assert!(authorize_workspace_effect(&app, &start).is_ok());

    let unavailable = authority(
        2,
        vec![(
            CapabilityId::BitBakeBuild,
            CapabilityState::Unavailable {
                reason: reason("connected backend cannot build"),
            },
            None,
        )],
    );
    let revalidated = install_workspace_compatibility(&mut app, unavailable).unwrap();
    assert!(revalidated.closed_dialog);
    assert!(app.active_dialog().is_none());
    assert_eq!(app.focus, crate::FocusTarget::Navigator);
    assert_eq!(app.navigator_selection, 3);
    let denied = authorize_workspace_effect(&app, &start).unwrap_err();
    assert!(denied.reason().contains("cannot build"));
    assert!(matches!(
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
        ),
        Err(WorkspaceCompatibilityError::StaleGeneration {
            current: 2,
            received: 1,
        })
    ));
    assert!(
        authorize_workspace_effect(&app, &start)
            .unwrap_err()
            .reason()
            .contains("cannot build")
    );
    assert!(
        authorize_workspace_effect(&app, &Effect::CopyToClipboard("still local".into())).is_ok()
    );
}
