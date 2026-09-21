use super::*;

#[test]
fn image_artifact_model_normalizes_exact_identities_fields_and_bounds() {
    let request = ImageArtifactRequest {
        generation: 1,
        machine: "qemux86-64".into(),
    };
    let mut preferred = artifact(
        "core-image-minimal",
        "/build/tmp/deploy/images/qemux86-64/core-image-minimal.ext4",
    );
    preferred.manifests = ImageArtifactField::Available(vec![
        "/build/tmp/deploy/images/qemux86-64/core-image-minimal.manifest".into(),
        "/build/tmp/deploy/images/qemux86-64/core-image-minimal.manifest".into(),
        "/outside/core-image-minimal.manifest".into(),
    ]);
    let mut duplicate = preferred.clone();
    duplicate.size_bytes = ImageArtifactField::Available(2_048);
    let mut wrong_machine = artifact(
        "core-image-base",
        "/build/tmp/deploy/images/qemux86-64/core-image-base.ext4",
    );
    wrong_machine.identity.machine = "qemuarm64".into();
    let overflow = artifact(
        "core-image-sato",
        "/build/tmp/deploy/images/qemux86-64/core-image-sato.wic",
    );
    let inventory = ImageArtifactInventory {
        machine: "qemux86-64".into(),
        deploy_directory: ImageArtifactField::Available(
            "/build/tmp/deploy/images/qemux86-64".into(),
        ),
        artifacts: vec![duplicate, overflow, wrong_machine, preferred],
    };
    let (inventory, report) = normalize_image_artifact_inventory(&request, inventory, 1);
    let inventory = inventory.unwrap();
    assert_eq!(inventory.artifacts.len(), 1);
    assert_eq!(
        inventory.artifacts[0].size_bytes,
        ImageArtifactField::Available(1_024)
    );
    assert_eq!(
        inventory.artifacts[0].manifests,
        ImageArtifactField::Available(vec![PathBuf::from(
            "/build/tmp/deploy/images/qemux86-64/core-image-minimal.manifest"
        )])
    );
    assert_eq!(report.duplicate_records, 1);
    assert_eq!(report.invalid_records, 1);
    assert_eq!(report.invalid_fields, 2);
    assert_eq!(report.truncated_records, 1);
    assert!(report.is_partial());
}
