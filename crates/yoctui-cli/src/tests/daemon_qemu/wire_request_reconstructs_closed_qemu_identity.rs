use super::*;

#[test]
fn wire_request_reconstructs_closed_qemu_identity() {
    let request = DaemonQemuRequest {
        machine: "qemux86-64".into(),
        image_machine: "qemux86-64".into(),
        image: "core-image-minimal".into(),
        image_path: "/tmp/core-image-minimal.ext4".into(),
        artifact_kind: "RootFilesystem".into(),
        kernel: None,
        rootfs: None,
        networking: "Slirp".into(),
        display: "Nographic".into(),
        serial: "Stdio".into(),
        memory_mib: 1024,
        extra_arguments: vec!["foo=bar".into()],
    };
    let (preview, cwd) = wire_preview(request, "/tmp/runqemu".into(), "/tmp/build".into()).unwrap();
    assert_eq!(preview.request.machine, "qemux86-64");
    assert_eq!(preview.argv[0], PathBuf::from("/tmp/runqemu"));
    assert!(preview.argv.contains(&PathBuf::from("qemuparams=-m 1024")));
    assert!(
        !preview
            .argv
            .iter()
            .any(|arg| arg.to_string_lossy().starts_with("qemumemory="))
    );
    assert_eq!(cwd, PathBuf::from("/tmp/build"));
}

#[test]
fn wire_openbmc_flash_keeps_preview_memory_snapshot_and_exact_boot_config() {
    let request = DaemonQemuRequest {
        machine: "romulus".into(),
        image_machine: "romulus".into(),
        image: "obmc-phosphor-image".into(),
        image_path: "/deploy/romulus/obmc-phosphor-image-romulus-20261009.static.mtd".into(),
        artifact_kind: "RootFilesystem".into(),
        kernel: None,
        rootfs: None,
        networking: "None".into(),
        display: "Nographic".into(),
        serial: "Stdio".into(),
        memory_mib: 512,
        extra_arguments: Vec::new(),
    };
    let (preview, _) = wire_preview(request, "/opt/runqemu".into(), "/build".into()).unwrap();
    assert_eq!(
        preview.argv[2],
        PathBuf::from("/deploy/romulus/obmc-phosphor-image-romulus-20261009.qemuboot.conf")
    );
    for argument in [
        "snapshot",
        "qemuparams=-m 512",
        "nonetwork",
        "nographic",
        "serialstdio",
    ] {
        assert!(preview.argv.contains(&PathBuf::from(argument)));
    }
    assert_eq!(&preview.argv[1..], preview.request.arguments());
}
