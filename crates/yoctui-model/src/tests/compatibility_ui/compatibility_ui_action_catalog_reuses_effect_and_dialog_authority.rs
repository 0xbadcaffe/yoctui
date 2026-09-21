use super::*;

#[test]
fn compatibility_ui_action_catalog_reuses_effect_and_dialog_authority() {
    let absent = WorkspaceCompatibilityState::default();
    let build = Effect::Start(crate::BuildRequest {
        targets: vec!["core-image-minimal".into()],
        task: None,
        force: false,
    });
    let build_action = compatibility_ui_effect_action_availability(&absent, &build);
    assert!(!build_action.enabled);
    assert_eq!(build_action.state, WorkspaceAvailabilityState::Unknown);

    let local_effect = Effect::CopyToClipboard("value".into());
    let local_action = compatibility_ui_effect_action_availability(&absent, &local_effect);
    assert!(local_action.enabled);
    assert_eq!(local_action.state, WorkspaceAvailabilityState::Available);

    let gated_dialog = compatibility_ui_dialog_action_availability(&absent, &Dialog::BuildOptions);
    assert!(!gated_dialog.enabled);
    assert_eq!(gated_dialog.state, WorkspaceAvailabilityState::Unknown);

    let local_dialog =
        compatibility_ui_dialog_action_availability(&absent, &Dialog::QuitConfirmation);
    assert!(local_dialog.enabled);
    assert_eq!(local_dialog.state, WorkspaceAvailabilityState::Available);
}
