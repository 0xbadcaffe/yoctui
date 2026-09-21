use super::*;

#[test]
fn image_artifact_model_rejects_request_inventory_identity_mismatch() {
    let request = ImageArtifactRequest {
        generation: 2,
        machine: "qemux86-64".into(),
    };
    let inventory = ImageArtifactInventory {
        machine: "qemuarm64".into(),
        deploy_directory: ImageArtifactField::Unavailable,
        artifacts: Vec::new(),
    };
    let (inventory, report) = normalize_image_artifact_inventory(&request, inventory, 10);
    assert!(inventory.is_none());
    assert_eq!(report.invalid_records, 1);
}
