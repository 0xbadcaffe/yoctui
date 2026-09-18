use super::*;

#[tokio::test]
async fn devtool_modify_completion_uses_authoritative_source_and_preserves_failures() {
    let directory =
        std::env::temp_dir().join(format!("yoctui-devtool-modify-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("main.c"), "int main(void) { return 0; }\n").unwrap();
    let identity = RecipeIdentity {
        name: "busybox".into(),
        file: PathBuf::from("/layers/meta/recipes-core/busybox/busybox.bb"),
    };
    let mut app = App::new(10, 1_000);
    app.workspace.recipes.push(yoctui_model::Recipe {
        name: identity.name.clone(),
        file: Some(identity.file.clone()),
        ..yoctui_model::Recipe::default()
    });
    let job_id = yoctui_model::BackgroundJobId(1_u64 << 63);
    let _ = update(
        &mut app,
        Action::QueueBackgroundJob(yoctui_model::BackgroundJobSpec {
            id: job_id,
            kind: yoctui_model::BackgroundJobKind::Devtool,
            title: "Devtool modify busybox".into(),
            context: yoctui_model::BackgroundJobContext {
                recipe: Some("busybox".into()),
                ..yoctui_model::BackgroundJobContext::default()
            },
            cancellation_supported: true,
            queued_at: SystemTime::UNIX_EPOCH,
        }),
    );
    let _ = update(
        &mut app,
        Action::StartBackgroundJob {
            id: job_id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::RunBackgroundJob { id: job_id });
    let _ = update(
        &mut app,
        Action::SucceedBackgroundJob {
            id: job_id,
            result: yoctui_model::BackgroundJobResult {
                summary: "Devtool completed successfully".into(),
                artifacts: vec![],
            },
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );

    apply_completed_devtool_modify_status(
        &mut app,
        yoctui_model::DevtoolStatus {
            identity: identity.clone(),
            capability: yoctui_model::DevtoolCapability::Available,
            workspace: DevtoolWorkspace::Present {
                source_path: directory.clone(),
                recipe_file: Some(identity.file.clone()),
            },
            git: yoctui_model::DevtoolGitState::NotRepository,
            error: None,
        },
    )
    .await;
    assert!(app.devtool_statuses.contains_key(&identity));
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::RecipeEditor(editor))
            if editor.recipe == "busybox"
                && editor.root == directory
                && editor.document.text.contains("int main")
    ));
    assert_eq!(
        app.background_jobs.get(job_id).unwrap().status,
        yoctui_model::BackgroundJobStatus::Succeeded
    );

    let _ = update(&mut app, Action::CloseRecipeEditor);
    let missing = directory.join("missing");
    apply_completed_devtool_modify_status(
        &mut app,
        yoctui_model::DevtoolStatus {
            identity: identity.clone(),
            capability: yoctui_model::DevtoolCapability::Available,
            workspace: DevtoolWorkspace::MissingDirectory {
                source_path: missing.clone(),
            },
            git: yoctui_model::DevtoolGitState::NotApplicable,
            error: None,
        },
    )
    .await;
    assert!(app.active_dialog().is_none());
    assert!(
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains(&missing.display().to_string()))
    );
    apply_completed_devtool_modify_status(
        &mut app,
        yoctui_model::DevtoolStatus {
            identity: identity.clone(),
            capability: yoctui_model::DevtoolCapability::Available,
            workspace: DevtoolWorkspace::NotMember,
            git: yoctui_model::DevtoolGitState::NotApplicable,
            error: Some(yoctui_model::DevtoolStatusError::DevtoolFailed {
                exit_code: Some(7),
                message: "status failed".into(),
            }),
        },
    )
    .await;
    assert!(
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("status refresh failed"))
    );
    apply_completed_devtool_modify_status(
        &mut app,
        yoctui_model::DevtoolStatus {
            identity: identity.clone(),
            capability: yoctui_model::DevtoolCapability::Available,
            workspace: DevtoolWorkspace::Present {
                source_path: missing,
                recipe_file: Some(identity.file),
            },
            git: yoctui_model::DevtoolGitState::NotApplicable,
            error: None,
        },
    )
    .await;
    assert!(
        app.notification
            .as_deref()
            .is_some_and(|message| message.contains("Could not list workspace files"))
    );
    assert_eq!(
        app.background_jobs.get(job_id).unwrap().status,
        yoctui_model::BackgroundJobStatus::Succeeded
    );
    fs::remove_dir_all(directory).unwrap();
}
