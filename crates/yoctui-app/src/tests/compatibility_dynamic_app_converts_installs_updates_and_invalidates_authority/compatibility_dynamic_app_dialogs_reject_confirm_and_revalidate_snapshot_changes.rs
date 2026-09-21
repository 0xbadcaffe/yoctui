use super::*;

#[test]
fn compatibility_dynamic_app_dialogs_reject_confirm_and_revalidate_snapshot_changes() {
    let request = yoctui_model::BuildRequest {
        targets: vec!["core-image-minimal".into()],
        task: None,
        force: false,
    };
    let mut app = yoctui_model::App::new(16, 4096);
    app.dialogs
        .push_front(yoctui_model::Dialog::RecipeTaskConfirmation(
            request.clone(),
        ));
    app.focus = yoctui_model::FocusTarget::Dialog;
    let presentation = yoctui_model::compatibility_ui_dialog_action_availability(
        &app.workspace_compatibility,
        app.active_dialog().unwrap(),
    );
    assert!(!presentation.enabled);
    assert_eq!(
        presentation.state,
        yoctui_model::WorkspaceAvailabilityState::Unknown
    );
    assert_eq!(
        compatibility_workspace_action(&mut app, yoctui_model::Action::ConfirmRecipeTask,),
        None
    );
    assert!(matches!(
        app.active_dialog(),
        Some(yoctui_model::Dialog::RecipeTaskConfirmation(_))
    ));

    yoctui_model::install_workspace_compatibility(&mut app, compatibility_workspace_authority(4))
        .unwrap();
    let presentation = yoctui_model::compatibility_ui_dialog_action_availability(
        &app.workspace_compatibility,
        app.active_dialog().unwrap(),
    );
    assert!(presentation.enabled);
    assert_eq!(
        compatibility_workspace_action(&mut app, yoctui_model::Action::ConfirmRecipeTask,),
        Some(yoctui_model::Effect::Start(request))
    );
    assert!(app.active_dialog().is_none());

    app.dialogs.push_front(yoctui_model::Dialog::BuildOptions);
    app.focus = yoctui_model::FocusTarget::Dialog;
    app.focus_return = Some(yoctui_model::FocusTarget::Inspector);
    let revalidated = yoctui_model::invalidate_workspace_compatibility(&mut app);
    assert!(revalidated.closed_dialog);
    assert!(app.active_dialog().is_none());
    assert_eq!(
        app.focus,
        yoctui_model::FocusTarget::Navigator,
        "Dashboard cannot restore a passive Inspector focus"
    );
    assert!(
        revalidated
            .reason
            .unwrap()
            .contains("current environment capability snapshot")
    );

    app.dialogs
        .push_front(yoctui_model::Dialog::QuitConfirmation);
    let local = yoctui_model::compatibility_ui_dialog_action_availability(
        &app.workspace_compatibility,
        app.active_dialog().unwrap(),
    );
    assert!(local.enabled);
    assert_eq!(
        local.state,
        yoctui_model::WorkspaceAvailabilityState::Available
    );
}
