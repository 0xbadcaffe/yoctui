use super::*;

#[test]
fn sdk_workflow_inventory_publication_native_and_lifecycle_are_correlated() {
    let mut app = sdk_workflow_app();
    let Some(Effect::GetSdkArtifacts(request)) =
        update(&mut app, Action::BeginSdkArtifactInventory)
    else {
        panic!("SDK inventory effect");
    };
    assert_eq!(
        update(&mut app, Action::BeginActiveSdkSessionCancellation),
        Some(Effect::CancelSdkArtifactOperation)
    );
    let stale = SdkArtifactInventoryRequest {
        generation: request.generation + 1,
        ..request.clone()
    };
    let ignored = app.background_jobs.ignored_transitions;
    let _ = update(
        &mut app,
        Action::SdkArtifactInventoryFailed {
            request: stale,
            message: "stale".into(),
        },
    );
    assert_eq!(app.background_jobs.ignored_transitions, ignored + 1);
    let artifact = SdkArtifact {
        identity: SdkArtifactIdentity {
            path: "/build/deploy/sdk/poky.sh".into(),
            size_bytes: 42,
            modified_unix_seconds: 7,
        },
        kind: SdkArtifactKind::Installer,
        sdk_kind: Some(SdkKind::Standard),
        machine: Some("qemux86-64".into()),
        host_tuple: Some("x86_64-pokysdk-linux".into()),
        target_tuple: Some("x86_64-poky-linux".into()),
        checksums: Vec::new(),
        manifests: Vec::new(),
        published: None,
    };
    let _ = update(
        &mut app,
        Action::SdkArtifactInventoryLoaded {
            request,
            artifacts: vec![artifact.clone()],
            limitations: vec!["one unrelated record skipped".into()],
        },
    );
    assert_eq!(app.sdk_artifact_selection, Some(artifact.identity.clone()));
    assert!(matches!(
        app.sdk_artifacts,
        SdkArtifactInventoryState::Partial { .. }
    ));
    assert_eq!(
        update(&mut app, Action::OpenSelectedSdkArtifact),
        Some(Effect::OpenInEditor(artifact.identity.path.clone()))
    );

    let _ = update(&mut app, Action::BeginSelectedSdkPublish);
    if let Some(Dialog::SdkPublishTomlEditor(editor)) = app.active_dialog_mut() {
        editor.text = "destination = \"/srv/sdk\"\n".into();
        editor.cursor = editor.text.len();
    }
    let _ = update(&mut app, Action::PreviewSdkPublish);
    let Some(Effect::StartSdkSession { id, operation }) =
        update(&mut app, Action::ConfirmSdkPublish)
    else {
        panic!("SDK publication effect");
    };
    assert!(matches!(
        operation,
        SdkOperation::Publish(SdkPublishRequest { destination, .. })
            if destination == Path::new("/srv/sdk")
    ));
    let _ = update(
        &mut app,
        Action::SdkSessionStarting {
            id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::SdkSessionRunning { id });
    let _ = update(
        &mut app,
        Action::AppendSdkSessionOutput {
            id,
            stream: SdkOutputStream::Stderr,
            line: "publication warning".into(),
            truncated: true,
            timestamp: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(
        &mut app,
        Action::CompleteSdkSession {
            id,
            exit_code: 0,
            artifacts: vec!["/srv/sdk/poky.sh".into()],
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    let session = app.sdk_session(id).unwrap();
    let job = app.background_jobs.get(session.background_job_id).unwrap();
    assert_eq!(job.status, BackgroundJobStatus::Succeeded);
    assert!(job.output[0].truncated);

    let _ = update(&mut app, Action::BeginSdkNative);
    let Some(Dialog::SdkNativeTomlEditor(editor)) = app.active_dialog_mut() else {
        panic!("SDK native dialog");
    };
    editor.text = "mode = \"run-native\"\nworkspace = \"/opt/sdk\"\nrecipe = \"cmake-native\"\ntool = \"cmake\"\narguments = \"--version\"\n".into();
    editor.cursor = editor.text.len();
    let _ = update(&mut app, Action::PreviewSdkNative);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::SdkNativeConfirmation(_))
    ));
    let _ = update(&mut app, Action::CancelSdkNativePreview);

    let _ = update(&mut app, Action::BeginSdkNative);
    if let Some(Dialog::SdkNativeTomlEditor(editor)) = app.active_dialog_mut() {
        editor.text = "mode = \"run-native\"\nworkspace = \"/opt/sdk\"\nrecipe = \"cmake-native\"\ntool = \"cmake\"\narguments = \"--version\"\n".into();
        editor.cursor = editor.text.len();
    }
    let _ = update(&mut app, Action::PreviewSdkNative);
    let Some(Effect::StartSdkSession { id: native_id, .. }) =
        update(&mut app, Action::ConfirmSdkNative)
    else {
        panic!("SDK native effect");
    };
    let _ = update(
        &mut app,
        Action::SdkSessionStarting {
            id: native_id,
            started_at: SystemTime::UNIX_EPOCH,
        },
    );
    let _ = update(&mut app, Action::SdkSessionRunning { id: native_id });
    let _ = update(&mut app, Action::BeginActiveSdkSessionCancellation);
    assert!(matches!(
        app.active_dialog(),
        Some(Dialog::SdkCancellationConfirmation(id)) if *id == native_id
    ));
    assert_eq!(
        update(&mut app, Action::ConfirmSdkSessionCancellation),
        Some(Effect::CancelSdkSession(native_id))
    );
    let _ = update(
        &mut app,
        Action::CancelSdkSession {
            id: native_id,
            exit_code: Some(130),
            finished_at: SystemTime::UNIX_EPOCH,
        },
    );
    assert_eq!(
        app.background_jobs
            .get(app.sdk_session(native_id).unwrap().background_job_id)
            .unwrap()
            .status,
        BackgroundJobStatus::Cancelled
    );
}
