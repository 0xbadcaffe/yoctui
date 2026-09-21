use super::*;

#[test]
fn image_artifact_adapter_classifies_only_uncompressed_wic_images() {
    assert_eq!(
        classify(Path::new("/deploy/image.wic")),
        ImageArtifactKind::Wic
    );
    assert_eq!(
        classify(Path::new("/deploy/image.direct")),
        ImageArtifactKind::Wic
    );
    assert_eq!(
        classify(Path::new("/deploy/image.wic.gz")),
        ImageArtifactKind::Other
    );
}
