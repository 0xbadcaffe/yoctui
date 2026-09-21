use super::*;

#[test]
fn test_workflow_test_runner_maps_keys_and_classifies_managed_bitbake_tests() {
    assert_eq!(
        testing_workspace_action(Input::Down),
        Some(Action::SelectTestFamily { delta: 1 })
    );
    assert_eq!(
        testing_workspace_action(Input::Char('r')),
        Some(Action::BeginSelectedTestLaunch)
    );
    assert_eq!(
        testing_workspace_action(Input::Char('x')),
        Some(Action::BeginActiveTestSessionCancellation)
    );
    assert_eq!(
        test_launch_dialog_action(false, Input::Down),
        Some(Action::SelectTestLaunchField { delta: 1 })
    );
    assert_eq!(
        test_launch_dialog_action(false, Input::Char('p')),
        Some(Action::PreviewTestLaunch)
    );
    assert_eq!(
        test_launch_dialog_action(true, Input::Char('x')),
        Some(Action::AppendTestLaunchField('x')),
        "editable launch fields must trap printable input"
    );
    assert_eq!(
        test_launch_dialog_action(true, Input::Enter),
        Some(Action::FinishTestLaunchFieldEdit)
    );
    assert_eq!(
        test_launch_dialog_action(true, Input::Esc),
        Some(Action::CancelTestLaunch),
        "Esc closes the launch dialog while a field is edited"
    );
    assert_eq!(
        test_launch_confirmation_action(Input::Enter),
        Some(Action::ConfirmTestLaunch)
    );
    assert_eq!(
        test_cancellation_confirmation_action(Input::Enter),
        Some(Action::ConfirmTestSessionCancellation)
    );

    let mut coordinator = BuildJobCoordinator::default();
    let actions = coordinator
        .queue_build(
            &BuildRequest {
                targets: vec!["core-image-minimal".into()],
                task: Some("testimage".into()),
                force: false,
            },
            SystemTime::UNIX_EPOCH,
        )
        .expect("valid testimage request");
    assert!(matches!(
        actions.first(),
        Some(Action::QueueBackgroundJob(BackgroundJobSpec {
            kind: BackgroundJobKind::Test,
            context: BackgroundJobContext {
                workspace: Some(Screen::Testing),
                task: Some(task),
                ..
            },
            ..
        })) if task == "testimage"
    ));
}
