use super::*;

#[tokio::test]
async fn devtool_target_deploy_refreshes_original_identity_and_retains_failures() {
    let identity = RecipeIdentity {
        name: "busybox".into(),
        file: PathBuf::from("/layers/meta/recipes-core/busybox/busybox.bb"),
    };
    let original = yoctui_model::DevtoolStatus {
        identity: identity.clone(),
        capability: yoctui_model::DevtoolCapability::Available,
        workspace: DevtoolWorkspace::Present {
            source_path: PathBuf::from("/build/workspace/sources/busybox"),
            recipe_file: Some(identity.file.clone()),
        },
        git: yoctui_model::DevtoolGitState::NotRepository,
        error: None,
    };
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Dashboard;
    app.devtool_statuses
        .insert(identity.clone(), original.clone());
    let operation = DevtoolOperation::DeployTarget {
        recipe: identity.name.clone(),
        target: "qemuarm".into(),
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
    assert_eq!(app.devtool_statuses.get(&identity), Some(&original));

    let refreshed = yoctui_model::DevtoolStatus {
        git: yoctui_model::DevtoolGitState::Available {
            repository_root: Some("/build/workspace/sources/busybox".into()),
            branch: Some("devtool".into()),
            upstream: None,
            ahead: 0,
            behind: 0,
            head: Some("abc123".into()),
            modified: 0,
            untracked: 0,
            conflicted: 0,
        },
        ..original
    };
    apply_completed_devtool_deploy_status(&mut app, refreshed.clone());
    assert_eq!(app.screen, Screen::Dashboard);
    assert_eq!(app.devtool_statuses.get(&identity), Some(&refreshed));
    assert!(
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("status was refreshed"))
    );
    apply_completed_devtool_deploy_status(
        &mut app,
        yoctui_model::DevtoolStatus {
            identity,
            capability: yoctui_model::DevtoolCapability::Available,
            workspace: DevtoolWorkspace::NotMember,
            git: yoctui_model::DevtoolGitState::NotApplicable,
            error: Some(yoctui_model::DevtoolStatusError::DevtoolFailed {
                exit_code: Some(9),
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
        app.background_jobs.get(failed_id).unwrap().status,
        yoctui_model::BackgroundJobStatus::Failed
    );
}
