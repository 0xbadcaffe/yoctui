use super::*;

#[test]
fn image_artifact_model_correlates_states_search_and_stable_selection() {
    let make_artifact = |image: &str, suffix: &str, kind| ImageArtifact {
        identity: ImageArtifactIdentity {
            machine: "qemux86-64".into(),
            image: image.into(),
            path: format!("/build/tmp/deploy/images/qemux86-64/{image}.{suffix}").into(),
        },
        kind,
        size_bytes: ImageArtifactField::Available(4_096),
        modified_unix_seconds: ImageArtifactField::Available(1_700_000_000),
        checksums: ImageArtifactField::Available(Vec::new()),
        manifests: ImageArtifactField::Available(Vec::new()),
        licenses: ImageArtifactField::Unavailable,
        spdx: ImageArtifactField::Unavailable,
        wic_files: ImageArtifactField::Available(Vec::new()),
    };
    let inventory = |artifacts| ImageArtifactInventory {
        machine: "qemux86-64".into(),
        deploy_directory: ImageArtifactField::Available(
            "/build/tmp/deploy/images/qemux86-64".into(),
        ),
        artifacts,
    };

    let mut app = App::new(20, 20_000);
    app.workspace
        .variables
        .insert("MACHINE".into(), "qemux86-64".into());
    let request = ImageArtifactRequest {
        generation: 1,
        machine: "qemux86-64".into(),
    };
    assert_eq!(
        update(&mut app, Action::BeginImageArtifactInventory),
        Some(Effect::GetImageArtifacts(request.clone()))
    );
    let minimal = make_artifact(
        "core-image-minimal",
        "rootfs.ext4",
        ImageArtifactKind::RootFilesystem,
    );
    let sato = make_artifact("core-image-sato", "wic", ImageArtifactKind::Wic);
    let _ = update(
        &mut app,
        Action::ImageArtifactInventoryLoaded {
            request: request.clone(),
            inventory: inventory(vec![sato.clone(), minimal.clone()]),
        },
    );
    assert!(matches!(
        app.image_artifacts,
        ImageArtifactInventoryState::Available { .. }
    ));
    assert_eq!(app.image_artifact_selection, Some(minimal.identity.clone()));
    let _ = update(&mut app, Action::SelectImageArtifact { delta: 1 });
    assert_eq!(app.image_artifact_selection, Some(sato.identity.clone()));

    assert_eq!(
        update(&mut app, Action::RefreshImageArtifactInventory),
        Some(Effect::GetImageArtifacts(ImageArtifactRequest {
            generation: 2,
            machine: "qemux86-64".into(),
        }))
    );
    let _ = update(
        &mut app,
        Action::ImageArtifactInventoryLoaded {
            request,
            inventory: inventory(vec![minimal.clone()]),
        },
    );
    assert!(matches!(
        app.image_artifacts,
        ImageArtifactInventoryState::Loading { .. }
    ));
    let request = ImageArtifactRequest {
        generation: 2,
        machine: "qemux86-64".into(),
    };
    let _ = update(
        &mut app,
        Action::ImageArtifactInventoryPartial {
            request: request.clone(),
            inventory: inventory(vec![minimal.clone(), sato.clone()]),
            limitations: vec!["checksum metadata unavailable".into()],
        },
    );
    assert!(matches!(
        app.image_artifacts,
        ImageArtifactInventoryState::Partial { .. }
    ));
    assert_eq!(app.image_artifact_selection, Some(sato.identity.clone()));

    let _ = update(&mut app, Action::BeginImageArtifactSearch);
    let _ = update(&mut app, Action::AppendImageArtifactQuery('m'));
    let _ = update(&mut app, Action::AppendImageArtifactQuery('i'));
    let _ = update(&mut app, Action::AppendImageArtifactQuery('n'));
    assert_eq!(app.filtered_image_artifacts(), vec![&minimal]);
    assert_eq!(app.image_artifact_selection, Some(minimal.identity.clone()));
    let _ = update(&mut app, Action::FinishImageArtifactSearch);

    app.image_artifact_query.clear();
    let failed_request = ImageArtifactRequest {
        generation: 3,
        machine: "qemux86-64".into(),
    };
    let _ = update(&mut app, Action::RefreshImageArtifactInventory);
    let _ = update(
        &mut app,
        Action::ImageArtifactInventoryFailed {
            request: failed_request.clone(),
            message: "deploy directory is unavailable".into(),
        },
    );
    assert!(matches!(
        app.image_artifacts,
        ImageArtifactInventoryState::Failed { ref request, .. } if request == &failed_request
    ));

    let empty_request = ImageArtifactRequest {
        generation: 4,
        machine: "qemux86-64".into(),
    };
    let _ = update(&mut app, Action::RefreshImageArtifactInventory);
    let _ = update(
        &mut app,
        Action::ImageArtifactInventoryLoaded {
            request: empty_request,
            inventory: inventory(Vec::new()),
        },
    );
    assert!(matches!(
        app.image_artifacts,
        ImageArtifactInventoryState::AvailableEmpty { .. }
    ));
    assert_eq!(app.image_artifact_selection, None);

    let invalid_request = ImageArtifactRequest {
        generation: 5,
        machine: "qemux86-64".into(),
    };
    let _ = update(&mut app, Action::RefreshImageArtifactInventory);
    let _ = update(
        &mut app,
        Action::ImageArtifactInventoryLoaded {
            request: invalid_request,
            inventory: ImageArtifactInventory {
                machine: "qemuarm64".into(),
                deploy_directory: ImageArtifactField::Unavailable,
                artifacts: Vec::new(),
            },
        },
    );
    assert!(matches!(
        app.image_artifacts,
        ImageArtifactInventoryState::Failed { .. }
    ));
}
