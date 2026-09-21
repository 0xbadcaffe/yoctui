use super::*;

#[test]
fn qemu_model_validates_exact_launch_identity_paths_and_options() {
    let artifact = qemu_model_artifact();
    let capability = QemuCapability::Available {
        executable: "/opt/poky/scripts/runqemu".into(),
        compatible_images: vec![artifact.identity.clone()],
    };
    let draft = QemuLaunchDraft::for_artifact(artifact.identity.clone(), artifact.kind);
    let first = draft.preview(&capability).expect("valid preview");
    let second = draft.preview(&capability).expect("deterministic preview");
    assert_eq!(first, second);
    assert_eq!(first.request.memory_mib, 1024);

    let mut invalid = draft.clone();
    invalid.machine = "other-machine".into();
    assert_eq!(
        invalid.preview(&capability),
        Err("runqemu machine and image identities must match")
    );
    invalid = draft.clone();
    invalid.rootfs = "relative/rootfs.ext4".into();
    assert!(invalid.preview(&capability).is_err());
    invalid = draft.clone();
    invalid.memory_mib = (MAX_QEMU_MEMORY_MIB + 1).to_string();
    assert!(invalid.preview(&capability).is_err());
    invalid = draft.clone();
    invalid.extra_arguments = "-- -display none".into();
    assert!(invalid.preview(&capability).is_err());
    invalid = draft;
    invalid.artifact_kind = ImageArtifactKind::Manifest;
    assert!(invalid.preview(&capability).is_err());
}
