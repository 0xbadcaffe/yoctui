use super::*;

#[tokio::test]
async fn devtool_publish_update_refreshes_original_identity_and_retains_failure() {
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
    let operation = DevtoolOperation::UpdateRecipe {
        recipe: identity.name.clone(),
    };
    let command = devtool_test_command("/bin/false".into(), &operation);
    let mut app = App::new(10, 1_000);
    app.screen = Screen::Dashboard;
    app.devtool_statuses
        .insert(identity.clone(), original.clone());
    let mut coordinator = DevtoolJobCoordinator::default();
    for action in coordinator
        .queue(operation, SystemTime::UNIX_EPOCH)
        .unwrap()
    {
        let _ = update(&mut app, action);
    }
    let id = coordinator.active_job_id().unwrap();
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
        app.background_jobs.get(id).unwrap().status,
        yoctui_model::BackgroundJobStatus::Failed
    );
    assert_eq!(app.devtool_statuses.get(&identity), Some(&original));

    let refreshed = yoctui_model::DevtoolStatus {
        identity: identity.clone(),
        capability: yoctui_model::DevtoolCapability::Available,
        workspace: DevtoolWorkspace::Present {
            source_path: PathBuf::from("/build/workspace/sources/busybox"),
            recipe_file: Some(identity.file.clone()),
        },
        git: yoctui_model::DevtoolGitState::Available {
            branch: Some("devtool".into()),
            head: Some("abc123".into()),
            modified: 0,
            untracked: 0,
            conflicted: 0,
        },
        error: None,
    };
    apply_completed_devtool_update_status(&mut app, refreshed.clone());
    assert_eq!(app.screen, Screen::Dashboard);
    assert_eq!(app.devtool_statuses.get(&identity), Some(&refreshed));
    assert!(
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("status was refreshed"))
    );
    apply_completed_devtool_update_status(
        &mut app,
        yoctui_model::DevtoolStatus {
            identity: identity.clone(),
            capability: yoctui_model::DevtoolCapability::Available,
            workspace: DevtoolWorkspace::NotMember,
            git: yoctui_model::DevtoolGitState::NotApplicable,
            error: Some(yoctui_model::DevtoolStatusError::DevtoolFailed {
                exit_code: Some(7),
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
        yoctui_model::BackgroundJobStatus::Failed
    );
}
