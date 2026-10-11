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

#[test]
fn image_artifact_adapter_classifies_timestamped_compressed_poky_filesystems() {
    for suffix in ["ext2.zst", "ext3.zst", "ext4.zst", "tar.zst", "cpio.gz"] {
        let path = format!("/deploy/core-image-minimal-qemux86-64.rootfs-20261010172613.{suffix}");
        assert_eq!(
            classify(Path::new(&path)),
            ImageArtifactKind::RootFilesystem
        );
    }
    for suffix in ["qemuboot.conf", "testdata.json"] {
        let path = format!("/deploy/core-image-minimal-qemux86-64.rootfs.{suffix}");
        assert_eq!(classify(Path::new(&path)), ImageArtifactKind::Other);
    }
}
