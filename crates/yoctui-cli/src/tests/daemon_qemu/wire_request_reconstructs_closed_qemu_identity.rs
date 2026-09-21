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
    assert_eq!(cwd, PathBuf::from("/tmp/build"));
}
