use super::*;

#[test]
fn image_artifact_model_typed_events_map_success_partial_and_failure() {
    let request = ImageArtifactRequest {
        generation: 9,
        machine: "qemux86-64".into(),
    };
    let artifact = ImageArtifact {
        identity: ImageArtifactIdentity {
            machine: request.machine.clone(),
            image: "core-image-minimal".into(),
            path: "/build/tmp/deploy/images/qemux86-64/core-image-minimal.wic".into(),
        },
        kind: ImageArtifactKind::Wic,
        size_bytes: ImageArtifactField::Available(8_192),
        modified_unix_seconds: ImageArtifactField::Available(1_700_000_000),
        checksums: ImageArtifactField::Unavailable,
        manifests: ImageArtifactField::Available(Vec::new()),
        licenses: ImageArtifactField::Unavailable,
        spdx: ImageArtifactField::Unavailable,
        wic_files: ImageArtifactField::Available(Vec::new()),
    };
    let inventory = ImageArtifactInventory {
        machine: request.machine.clone(),
        deploy_directory: ImageArtifactField::Available(
            "/build/tmp/deploy/images/qemux86-64".into(),
        ),
        artifacts: vec![artifact],
    };
    assert_eq!(
        model_action_from_backend_event(BackendEvent::ImageArtifacts {
            request: request.clone(),
            inventory: inventory.clone(),
            limitations: Vec::new(),
        }),
        Some(Action::ImageArtifactInventoryLoaded {
            request: request.clone(),
            inventory: inventory.clone(),
        })
    );
    assert_eq!(
        model_action_from_backend_event(BackendEvent::ImageArtifacts {
            request: request.clone(),
            inventory: inventory.clone(),
            limitations: vec!["license metadata unavailable".into()],
        }),
        Some(Action::ImageArtifactInventoryPartial {
            request: request.clone(),
            inventory,
            limitations: vec!["license metadata unavailable".into()],
        })
    );
    assert_eq!(
        model_action_from_backend_event(BackendEvent::ImageArtifactsFailed {
            request: request.clone(),
            message: "deploy directory is missing".into(),
        }),
        Some(Action::ImageArtifactInventoryFailed {
            request,
            message: "deploy directory is missing".into(),
        })
    );
}
