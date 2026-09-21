use super::*;

fn artifact(image: &str, path: &str) -> ImageArtifact {
    ImageArtifact {
        identity: ImageArtifactIdentity {
            machine: "qemux86-64".into(),
            image: image.into(),
            path: path.into(),
        },
        kind: ImageArtifactKind::RootFilesystem,
        size_bytes: ImageArtifactField::Available(1_024),
        modified_unix_seconds: ImageArtifactField::Available(1_700_000_000),
        checksums: ImageArtifactField::Available(Vec::new()),
        manifests: ImageArtifactField::Available(Vec::new()),
        licenses: ImageArtifactField::Available(Vec::new()),
        spdx: ImageArtifactField::Available(Vec::new()),
        wic_files: ImageArtifactField::Available(Vec::new()),
    }
}

mod image_artifact_model_normalizes_exact_identities_fields_and_bounds;

mod image_artifact_model_rejects_request_inventory_identity_mismatch;

mod ux_image_preview_policy_is_transport_invariant_and_never_fabricates_raster_authority;
