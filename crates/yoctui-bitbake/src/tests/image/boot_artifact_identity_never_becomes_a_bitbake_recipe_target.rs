use super::*;

#[test]
fn boot_artifact_identity_never_becomes_a_bitbake_recipe_target() {
    assert_eq!(
        image_target(
            Path::new("/deploy/bzImage--6.18.24+git0-r0-qemux86-64-20260831042030.bin"),
            "qemux86-64",
            ImageArtifactKind::Kernel,
        ),
        "kernel"
    );
    assert_eq!(
        image_target(
            Path::new("/deploy/u-boot-qemux86-64-20260831042030.bin"),
            "qemux86-64",
            ImageArtifactKind::Bootloader,
        ),
        "bootloader"
    );
}
