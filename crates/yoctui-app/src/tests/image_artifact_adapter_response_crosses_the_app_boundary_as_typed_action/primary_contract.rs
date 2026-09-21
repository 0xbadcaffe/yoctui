use super::*;

#[test]
fn image_artifact_adapter_response_crosses_the_app_boundary_as_typed_action() {
    let request = ImageArtifactRequest {
        generation: 12,
        machine: "qemux86-64".into(),
    };
    let inventory = ImageArtifactInventory {
        machine: request.machine.clone(),
        deploy_directory: ImageArtifactField::Available(
            "/build/tmp/deploy/images/qemux86-64".into(),
        ),
        artifacts: Vec::new(),
    };
    let event: BackendEvent = yoctui_bitbake::ImageArtifactResponse {
        request: request.clone(),
        inventory: inventory.clone(),
        limitations: vec!["one symlink was not followed".into()],
    }
    .into();
    assert_eq!(
        model_action_from_backend_event(event),
        Some(Action::ImageArtifactInventoryPartial {
            request,
            inventory,
            limitations: vec!["one symlink was not followed".into()],
        })
    );
}
