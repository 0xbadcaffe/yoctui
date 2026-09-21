use super::*;

#[test]
fn compatibility_dynamic_app_actions_follow_installed_authority_and_runtime_gate() {
    let mut app = yoctui_model::App::new(16, 4096);
    yoctui_model::install_workspace_compatibility(&mut app, compatibility_workspace_authority(3))
        .unwrap();

    let dashboard = yoctui_model::compatibility_ui_workspace_action_presentations(
        &app.workspace_compatibility,
        yoctui_model::WorkspaceDestination::Dashboard,
    );
    let build = dashboard
        .iter()
        .find(|action| action.id == "dashboard.build")
        .unwrap();
    assert!(build.availability.enabled);
    assert_eq!(
        build.availability.implementations,
        [(
            yoctui_model::CapabilityId::BitBakeBuild,
            "bitbake.build.command".into()
        )]
    );
    let cancel = dashboard
        .iter()
        .find(|action| action.id == "dashboard.cancel")
        .unwrap();
    assert!(!cancel.availability.enabled);
    assert_eq!(
        cancel.availability.state,
        yoctui_model::WorkspaceAvailabilityState::Unknown
    );

    app.build.status = yoctui_model::BuildStatus::Running;
    let before = app.build.clone();
    assert_eq!(
        compatibility_workspace_action(&mut app, yoctui_model::Action::Cancel),
        None
    );
    assert!(matches!(
        app.active_dialog(),
        Some(yoctui_model::Dialog::BuildCancellationConfirmation)
    ));
    assert_eq!(
        compatibility_workspace_action(&mut app, yoctui_model::Action::ConfirmBuildCancellation,),
        None
    );
    assert_eq!(app.build, before);
    assert!(
        app.notification
            .as_deref()
            .unwrap()
            .contains("bitbake.cancellation")
    );

    let images = yoctui_model::compatibility_ui_workspace_action_presentations(
        &app.workspace_compatibility,
        yoctui_model::WorkspaceDestination::Images,
    );
    assert!(
        images
            .iter()
            .find(|action| action.id == "images.device_write")
            .unwrap()
            .availability
            .enabled
    );
    yoctui_model::invalidate_workspace_compatibility(&mut app);
    let unloaded = yoctui_model::compatibility_ui_workspace_action_presentations(
        &app.workspace_compatibility,
        yoctui_model::WorkspaceDestination::Dashboard,
    );
    assert_eq!(
        unloaded
            .iter()
            .find(|action| action.id == "dashboard.build")
            .unwrap()
            .availability
            .state,
        yoctui_model::WorkspaceAvailabilityState::Unknown
    );
}
