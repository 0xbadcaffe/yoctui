use super::*;

#[cfg(unix)]
#[tokio::test]
async fn rootfs_completed_metadata_cannot_install_after_authority_loss() {
    let mut app = App::new(16, 4096);
    let request = RootfsCompositionRequest {
        generation: 1,
        image: yoctui_model::ImageArtifactIdentity {
            machine: "machine".into(),
            image: "image".into(),
            path: "/build/image".into(),
        },
    };
    app.rootfs_composition = yoctui_model::RootfsCompositionState::Loading {
        request: request.clone(),
    };
    app.rootfs_request_generation = 1;
    let result_request = request.clone();
    let handle = tokio::spawn(async move {
        BackendEvent::RootfsCompositionUnavailable {
            request: result_request,
            reason: "old result must not install".into(),
        }
    });
    let mut operation = Some(RootfsCompositionBackgroundOperation {
        request,
        authority: Some(yoctui_protocol::rootfs::RootfsSourcesRequestData {
            request: yoctui_protocol::rootfs::RootfsCompositionRequestData {
                generation: 1,
                image: yoctui_protocol::rootfs::RootfsImageIdentityData {
                    machine: "machine".into(),
                    image: "image".into(),
                    path: "/build/image".into(),
                },
            },
            daemon_instance_id: yoctui_protocol::daemon::DaemonInstanceId([1; 16]),
            compatibility_generation: 1,
        }),
        _cancellation: RootfsCompositionCancellation::default(),
        handle,
    });
    tokio::time::timeout(Duration::from_secs(1), async {
        while operation.is_some() {
            poll_rootfs_composition_operation(&mut app, &mut operation).await;
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(
        matches!(app.rootfs_composition, yoctui_model::RootfsCompositionState::Failed { ref message, .. } if message.contains("authority changed"))
    );
}
