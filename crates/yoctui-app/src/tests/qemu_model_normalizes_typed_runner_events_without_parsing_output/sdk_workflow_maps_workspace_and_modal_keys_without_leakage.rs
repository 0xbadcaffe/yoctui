use super::*;

#[test]
fn sdk_workflow_maps_workspace_and_modal_keys_without_leakage() {
    assert_eq!(
        sdk_workspace_action(false, Input::Char('s')),
        Some(Action::BeginSdkBuild(SdkBuildAction::Populate(
            SdkKind::Standard
        )))
    );
    assert_eq!(
        sdk_workspace_action(false, Input::Char('E')),
        Some(Action::BeginSdkBuild(SdkBuildAction::Populate(
            SdkKind::Extensible
        )))
    );
    assert_eq!(
        sdk_workspace_action(false, Input::Char('t')),
        Some(Action::BeginSdkBuild(SdkBuildAction::Test(
            SdkKind::Standard
        )))
    );
    assert_eq!(
        sdk_workspace_action(false, Input::Char('T')),
        Some(Action::BeginSdkBuild(SdkBuildAction::Test(
            SdkKind::Extensible
        )))
    );
    assert_eq!(
        sdk_workspace_action(true, Input::Char('x')),
        Some(Action::AppendSdkArtifactQuery('x'))
    );
    assert_eq!(
        sdk_workspace_action(false, Input::Char('o')),
        Some(Action::OpenSelectedSdkArtifact)
    );
    assert_eq!(
        sdk_build_confirmation_action(Input::Enter),
        Some(Action::ConfirmSdkBuild)
    );
    assert_eq!(
        sdk_publish_dialog_action(Input::Char('P')),
        Some(Action::AppendSdkPublishDestination('P')),
        "publication modal input must not leak to the SDK workspace"
    );
    assert_eq!(
        sdk_publish_confirmation_action(Input::Enter),
        Some(Action::ConfirmSdkPublish)
    );
    assert_eq!(
        sdk_native_dialog_action(false, Input::Esc),
        Some(Action::CancelSdkNative)
    );
    assert_eq!(
        sdk_native_dialog_action(false, Input::Char('p')),
        Some(Action::PreviewSdkNative)
    );
    assert_eq!(
        sdk_native_dialog_action(false, Input::Down),
        Some(Action::SelectSdkNativeField { delta: 1 })
    );
    assert_eq!(
        sdk_native_dialog_action(false, Input::Enter),
        Some(Action::ActivateSdkNativeField)
    );
    assert_eq!(
        sdk_native_dialog_action(true, Input::Char('x')),
        Some(Action::AppendSdkNativeField('x'))
    );
    assert_eq!(
        sdk_native_dialog_action(true, Input::Enter),
        Some(Action::FinishSdkNativeFieldEdit)
    );
    assert_eq!(
        sdk_native_dialog_action(true, Input::Esc),
        Some(Action::CancelSdkNative),
        "Esc closes the dialog even while a field is being edited"
    );
    assert_eq!(
        sdk_cancellation_confirmation_action(Input::Enter),
        Some(Action::ConfirmSdkSessionCancellation)
    );
    let id = SdkSessionId(7);
    assert_eq!(
        sdk_actions_for_runner_event(id, SdkToolRunnerEvent::Started, SystemTime::UNIX_EPOCH),
        vec![
            Action::SdkSessionStarting {
                id,
                started_at: SystemTime::UNIX_EPOCH
            },
            Action::SdkSessionRunning { id }
        ]
    );
    assert!(matches!(
        sdk_actions_for_runner_event(
            id,
            SdkToolRunnerEvent::TimedOut {
                forced: true,
                exit_code: None
            },
            SystemTime::UNIX_EPOCH
        )
        .as_slice(),
        [Action::FailSdkSession {
            id: SdkSessionId(7),
            message,
            exit_code: None,
            ..
        }] if message.contains("timed out") && message.contains("forced")
    ));
}
