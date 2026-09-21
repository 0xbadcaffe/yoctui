use super::*;

#[test]
fn recipe_bitbake_action_maps_standard_and_forced_task_controls() {
    assert_eq!(
        recipes_workspace_action(false, Input::Char('f')),
        Some(Action::BeginSelectedRecipeForceTask)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('v')),
        Some(Action::BeginSelectedRecipeDevshell)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('K')),
        Some(Action::BeginSelectedRecipeDiffconfig)
    );
    assert_eq!(
        recipes_workspace_action(false, Input::Char('z')),
        Some(Action::BeginSelectedRecipeDiffsigs)
    );
    let request = BuildRequest {
        targets: vec!["busybox".into()],
        task: Some("compile".into()),
        force: true,
    };
    let mut coordinator = BuildJobCoordinator::default();
    let actions = coordinator
        .queue_build(&request, SystemTime::UNIX_EPOCH)
        .unwrap();
    assert!(matches!(
        &actions[0],
        Action::QueueBackgroundJob(spec)
            if spec.context.target.as_deref() == Some("busybox")
                && spec.context.task.as_deref() == Some("compile")
    ));
    assert!(
        coordinator
            .queue_build(&request, SystemTime::UNIX_EPOCH)
            .is_none()
    );
}
