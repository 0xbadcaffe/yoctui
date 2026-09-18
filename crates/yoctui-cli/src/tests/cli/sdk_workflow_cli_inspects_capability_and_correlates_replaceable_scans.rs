use super::*;

#[cfg(unix)]
#[tokio::test]
async fn sdk_workflow_cli_inspects_capability_and_correlates_replaceable_scans() {
    use std::os::unix::fs::symlink;

    let (directory, _, artifact_adapter, tool_adapter, mut app) =
        sdk_workflow_fixture("scan", "exit 0", "exit 0", "exit 0");
    let mut capability = None;
    begin_sdk_capability_operation(
        &mut app,
        Some(&tool_adapter),
        &mut capability,
        Effect::InspectSdkTools,
    );
    tokio::time::timeout(Duration::from_secs(2), async {
        while capability.is_some() {
            poll_sdk_capability_operation(&mut app, &mut capability).await;
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(matches!(
        app.sdk_tool_capability,
        SdkToolCapability::Available {
            publish: Some(_),
            find_sysroot: Some(_),
            run_native: Some(_)
        }
    ));

    let deploy = PathBuf::from(app.workspace.variables.get("SDK_DEPLOY").unwrap());
    symlink("/outside-sdk", deploy.join("ignored-link")).unwrap();
    let first_effect = update(&mut app, Action::BeginSdkArtifactInventory).unwrap();
    let Effect::GetSdkArtifacts(first_request) = first_effect.clone() else {
        panic!("expected SDK scan");
    };
    let mut scan = None;
    begin_sdk_artifact_operation(&mut app, Some(&artifact_adapter), &mut scan, first_effect);

    let replacement = SdkArtifactInventoryRequest {
        generation: first_request.generation + 1,
        ..first_request.clone()
    };
    app.sdk_artifacts = yoctui_model::SdkArtifactInventoryState::Loading {
        request: replacement.clone(),
    };
    begin_sdk_artifact_operation(
        &mut app,
        Some(&artifact_adapter),
        &mut scan,
        Effect::GetSdkArtifacts(replacement.clone()),
    );
    let ignored_before = app.background_jobs.ignored_transitions;
    let _ = update(
        &mut app,
        Action::SdkArtifactInventoryLoaded {
            request: first_request,
            artifacts: Vec::new(),
            limitations: Vec::new(),
        },
    );
    assert_eq!(app.background_jobs.ignored_transitions, ignored_before + 1);
    sdk_workflow_poll_scan(&mut app, &mut scan).await;
    assert!(matches!(
        &app.sdk_artifacts,
        yoctui_model::SdkArtifactInventoryState::Partial {
            request,
            artifacts,
            limitations
        } if request == &replacement
            && artifacts.iter().any(|artifact| artifact.kind == yoctui_model::SdkArtifactKind::Installer)
            && limitations.iter().any(|limitation| limitation.contains("symlink"))
    ));

    let failure_request = SdkArtifactInventoryRequest {
        generation: replacement.generation + 1,
        ..replacement
    };
    app.sdk_artifacts = yoctui_model::SdkArtifactInventoryState::Loading {
        request: failure_request.clone(),
    };
    begin_sdk_artifact_operation(
        &mut app,
        None,
        &mut scan,
        Effect::GetSdkArtifacts(failure_request.clone()),
    );
    assert!(matches!(
        &app.sdk_artifacts,
        yoctui_model::SdkArtifactInventoryState::Failed { request, message }
            if request == &failure_request && message.contains("SDK_DEPLOY")
    ));

    let cancel_effect = update(&mut app, Action::RefreshSdkArtifactInventory).unwrap();
    begin_sdk_artifact_operation(&mut app, Some(&artifact_adapter), &mut scan, cancel_effect);
    let Some(Effect::CancelSdkArtifactOperation) =
        update(&mut app, Action::BeginActiveSdkSessionCancellation)
    else {
        panic!("expected independently routed SDK scan cancellation");
    };
    assert!(scan.as_ref().unwrap().cancellation.cancel());
    sdk_workflow_poll_scan(&mut app, &mut scan).await;
    assert!(matches!(
        &app.sdk_artifacts,
        yoctui_model::SdkArtifactInventoryState::Failed { message, .. }
            if message.contains("cancelled")
    ));
    fs::remove_dir_all(directory).unwrap();
}
