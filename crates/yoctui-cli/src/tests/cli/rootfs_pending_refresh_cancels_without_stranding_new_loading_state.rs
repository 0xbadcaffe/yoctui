use super::*;

#[cfg(unix)]
#[tokio::test]
async fn rootfs_pending_refresh_cancels_without_stranding_new_loading_state() {
    let mut app = App::new(16, 4096);
    let request = RootfsCompositionRequest {
        generation: 2,
        image: yoctui_model::ImageArtifactIdentity {
            machine: "machine".into(),
            image: "image".into(),
            path: "/build/image".into(),
        },
    };
    app.rootfs_composition = yoctui_model::RootfsCompositionState::Loading {
        request: request.clone(),
    };
    app.rootfs_request_generation = 2;
    let cancellation = RootfsCompositionCancellation::default();
    let mut operation = Some(RootfsCompositionBackgroundOperation {
        request: RootfsCompositionRequest {
            generation: 1,
            ..request.clone()
        },
        authority: None,
        _cancellation: cancellation.clone(),
        handle: tokio::spawn(std::future::pending()),
    });
    let mut backend = ProcessBackend::new("/build".into());
    tokio::time::timeout(
        Duration::from_millis(100),
        begin_rootfs_composition_operation(
            &mut backend,
            &mut app,
            Path::new("/build"),
            &mut operation,
            Effect::GetRootfsComposition(request),
            true,
        ),
    )
    .await
    .unwrap();
    assert!(cancellation.is_cancelled());
    assert!(matches!(
        app.rootfs_composition,
        yoctui_model::RootfsCompositionState::Failed { .. }
    ));
    operation.as_ref().unwrap().handle.abort();
}
