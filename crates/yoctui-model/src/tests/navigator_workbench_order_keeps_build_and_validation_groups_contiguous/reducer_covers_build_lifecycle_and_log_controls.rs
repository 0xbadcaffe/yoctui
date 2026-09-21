use super::*;

#[test]
fn reducer_covers_build_lifecycle_and_log_controls() {
    let mut app = App::new(10, 1_000);
    assert!(
        update(
            &mut app,
            Action::Start(BuildRequest {
                targets: vec!["bad target".into()],
                task: None,
                force: false,
            }),
        )
        .is_none()
    );
    assert!(app.notification.is_some());
    let request = BuildRequest {
        targets: vec!["busybox".into()],
        task: Some("compile".into()),
        force: false,
    };
    assert_eq!(
        update(&mut app, Action::Start(request.clone())),
        Some(Effect::Start(request))
    );
    let _ = update(&mut app, Action::BuildStarted);
    let id = TaskId("busybox:do_compile".into());
    let _ = update(
        &mut app,
        Action::TaskStarted(TaskInfo {
            id: id.clone(),
            recipe: "busybox".into(),
            task: "do_compile".into(),
            progress: None,
            ..TaskInfo::default()
        }),
    );
    let _ = update(
        &mut app,
        Action::TaskProgress {
            id: id.clone(),
            progress: Some(50),
        },
    );
    let _ = update(&mut app, Action::TaskCompleted { id, success: true });
    assert_eq!(update(&mut app, Action::Cancel), None);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::BuildCancellationConfirmation)
    ));
    assert_eq!(
        update(&mut app, Action::ConfirmBuildCancellation),
        Some(Effect::Cancel)
    );
    let _ = update(
        &mut app,
        Action::BuildCompleted {
            success: false,
            exit_code: Some(1),
        },
    );
    assert_eq!(app.build.status, BuildStatus::Failed);
    assert_eq!(app.build.exit_code, Some(1));
    let _ = update(&mut app, Action::Open(Screen::Logs));
    let _ = update(&mut app, Action::BeginLogSearch);
    let _ = update(&mut app, Action::AppendLogQuery('x'));
    let _ = update(&mut app, Action::BackspaceLogQuery);
    let _ = update(&mut app, Action::FinishLogSearch);
    let _ = update(&mut app, Action::ScrollLogsHorizontally { delta: 5 });
    let _ = update(&mut app, Action::ScrollLogsHorizontally { delta: -5 });
    let _ = update(
        &mut app,
        Action::Failure(AppError::new("test", "failure", "retry")),
    );
    let _ = update(&mut app, Action::DismissNotification);
    assert!(app.notification.is_none());
}
