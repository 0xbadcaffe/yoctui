use super::*;

#[test]
fn daemon_client_rootfs_uses_selected_manifest_and_workspace_pkgdata() {
    use yoctui_model::{
        ImageArtifact, ImageArtifactField, ImageArtifactIdentity, ImageArtifactInventory,
        ImageArtifactInventoryState, ImageArtifactKind, ImageArtifactRequest,
    };

    let image = ImageArtifactIdentity {
        machine: "qemux86-64".into(),
        image: "core-image-minimal".into(),
        path: "/deploy/core-image-minimal.rootfs.ext4".into(),
    };
    let manifest: PathBuf = "/deploy/core-image-minimal.rootfs.manifest".into();
    let pkgdata: PathBuf = "/build/tmp/pkgdata/qemux86-64".into();
    let mut app = yoctui_model::App::new(64, 64 * 1024);
    app.workspace
        .variables
        .insert("PKGDATA_DIR".into(), pkgdata.display().to_string());
    app.image_artifacts = ImageArtifactInventoryState::Available {
        request: ImageArtifactRequest {
            generation: 1,
            machine: "qemux86-64".into(),
        },
        inventory: ImageArtifactInventory {
            machine: "qemux86-64".into(),
            deploy_directory: ImageArtifactField::Available("/deploy".into()),
            artifacts: vec![ImageArtifact {
                identity: image.clone(),
                kind: ImageArtifactKind::RootFilesystem,
                size_bytes: ImageArtifactField::Available(1),
                modified_unix_seconds: ImageArtifactField::Unavailable,
                checksums: ImageArtifactField::Unavailable,
                manifests: ImageArtifactField::Available(vec![manifest.clone()]),
                licenses: ImageArtifactField::Unavailable,
                spdx: ImageArtifactField::Unavailable,
                wic_files: ImageArtifactField::Unavailable,
            }],
        },
    };
    let request = RootfsCompositionRequest {
        generation: 1,
        image,
    };
    let sources =
        client_rootfs_composition_sources(&app, &request, None, None, Some("/gone".into()));
    assert_eq!(sources.manifest, Some(manifest));
    assert_eq!(sources.pkgdata_directory, Some(pkgdata));
    assert_eq!(sources.image_rootfs, Some("/gone".into()));
}
