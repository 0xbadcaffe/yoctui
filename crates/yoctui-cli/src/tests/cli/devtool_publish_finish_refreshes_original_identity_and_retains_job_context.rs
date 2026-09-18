use super::*;

#[tokio::test]
async fn devtool_publish_finish_refreshes_original_identity_and_retains_job_context() {
    let identity = RecipeIdentity {
        name: "busybox".into(),
        file: PathBuf::from("/layers/meta/recipes-core/busybox/busybox.bb"),
    };
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Dashboard;
    let id = yoctui_model::BackgroundJobId((1_u64 << 63) + 100);
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(yoctui_model::BackgroundJobSpec {
            id,
            kind: yoctui_model::BackgroundJobKind::Devtool,
            title: "Devtool finish busybox".into(),
            context: yoctui_model::BackgroundJobContext {
                recipe: Some("busybox".into()),
                path: Some(PathBuf::from("/layers/meta-demo")),
                ..yoctui_model::BackgroundJobContext::default()
            },
            cancellation_supported: true,
            queued_at: SystemTime::UNIX_EPOCH,
        }),
    );
    let _ = update(
        &mut app,
        Action::StartBackgroundJob {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::RunBackgroundJob { id });
    let _ = update(
        &mut app,
        Action::SucceedBackgroundJob {
            id,
            result: yoctui_model::BackgroundJobResult {
                summary: "Devtool completed successfully".into(),
                artifacts: vec![],
            },
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    let refreshed = yoctui_model::DevtoolStatus {
        identity: identity.clone(),
        capability: yoctui_model::DevtoolCapability::Available,
        workspace: DevtoolWorkspace::NotMember,
        git: yoctui_model::DevtoolGitState::NotApplicable,
        error: None,
    };
    apply_completed_devtool_finish_status(&mut app, refreshed.clone());
    assert_eq!(app.devtool_statuses.get(&identity), Some(&refreshed));
    assert_eq!(app.screen, Screen::Dashboard);
    assert!(
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("status was refreshed"))
    );
    apply_completed_devtool_finish_status(
        &mut app,
        yoctui_model::DevtoolStatus {
            identity: identity.clone(),
            capability: yoctui_model::DevtoolCapability::Available,
            workspace: DevtoolWorkspace::NotMember,
            git: yoctui_model::DevtoolGitState::NotApplicable,
            error: Some(yoctui_model::DevtoolStatusError::DevtoolFailed {
                exit_code: Some(5),
                message: "refresh failed".into(),
            }),
        },
    );
    assert!(
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("status refresh failed"))
    );
    assert_eq!(
        app.background_jobs.get(id).unwrap().status,
        yoctui_model::BackgroundJobStatus::Succeeded
    );

    let operation = DevtoolOperation::Finish {
        recipe: "busybox".into(),
        destination: PathBuf::from("/layers/meta-demo"),
    };
    let command = devtool_test_command("/bin/false".into(), &operation);
    let mut coordinator = DevtoolJobCoordinator::default();
    for action in coordinator
        .queue(operation, SystemTime::UNIX_EPOCH)
        .unwrap()
    {
        let _ = update(&mut app, action);
    }
    let failed_id = coordinator.active_job_id().unwrap();
    let mut started = DevtoolJobRunner::new(std::env::temp_dir());
    started.start(command).await.unwrap();
    let mut runner = Some(started);
    tokio::time::timeout(Duration::from_secs(2), async {
        while runner.is_some() {
            assert!(
                poll_devtool_job(&mut app, &mut coordinator, &mut runner)
                    .await
                    .is_none()
            );
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        app.background_jobs.get(failed_id).unwrap().status,
        yoctui_model::BackgroundJobStatus::Failed
    );
}
