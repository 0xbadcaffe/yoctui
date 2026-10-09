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

#[test]
fn image_artifact_adapter_classifies_openbmc_flash_and_compressed_squashfs() {
    for suffix in [
        "static.mtd",
        "squashfs-xz",
        "squashfs-lzo",
        "squashfs-lz4",
        "squashfs-zst",
    ] {
        assert_eq!(
            classify(Path::new(&format!(
                "/deploy/obmc-phosphor-image-romulus.{suffix}"
            ))),
            ImageArtifactKind::RootFilesystem
        );
    }
    for name in [
        "board.dtb",
        "image.qemuboot.conf",
        "image.static.mtd.sha256",
    ] {
        assert_ne!(classify(Path::new(name)), ImageArtifactKind::RootFilesystem);
    }
}
