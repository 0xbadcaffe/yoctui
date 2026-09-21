use super::*;

#[test]
fn sdk_workflow_navigates_and_previews_exact_managed_builds() {
    let mut app = sdk_workflow_app();
    let sdk_index = NAVIGATOR_SCREENS
        .iter()
        .position(|screen| *screen == Screen::Sdk)
        .unwrap();
    app.focus = FocusTarget::Navigator;
    app.navigator_selection = sdk_index;
    app.sdk_tool_capability = SdkToolCapability::NotInspected;
    assert_eq!(
        update(&mut app, Action::ActivateNavigator),
        Some(Effect::InspectSdkTools)
    );
    assert_eq!(app.screen, Screen::Sdk);
    let _ = update(
        &mut app,
        Action::BeginSdkBuild(SdkBuildAction::Populate(SdkKind::Extensible)),
    );
    let Some(Dialog::SdkBuildConfirmation(preview)) = app.active_dialog() else {
        panic!("SDK build confirmation");
    };
    assert_eq!(preview.machine, "qemux86-64");
    assert_eq!(preview.distro, "poky");
    assert_eq!(preview.request.task.as_deref(), Some("populate_sdk_ext"));
    assert_eq!(
        update(&mut app, Action::ConfirmSdkBuild),
        Some(Effect::Start(BuildRequest {
            targets: vec!["core-image-minimal".into()],
            task: Some("populate_sdk_ext".into()),
            force: false,
        }))
    );
    assert!(app.active_dialog().is_none());
}
