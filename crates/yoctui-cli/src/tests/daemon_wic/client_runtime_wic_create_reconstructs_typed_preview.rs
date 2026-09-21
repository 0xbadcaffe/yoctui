use super::*;

#[test]
fn client_runtime_wic_create_reconstructs_typed_preview() {
    let request = DaemonWicCreateRequest {
        machine: "qemux86-64".into(),
        image: "core-image-minimal".into(),
        kickstart_name: "directdisk".into(),
        kickstart_path: None,
        output_directory: "/tmp/wic-output".into(),
        generate_bmap: true,
        compression: "Gzip".into(),
    };
    let (preview, output) = wire_preview(request, "/usr/bin/wic".into()).unwrap();
    assert_eq!(preview.request.image, "core-image-minimal");
    assert_eq!(output, PathBuf::from("/tmp/wic-output"));
    assert!(preview.argv.iter().any(|arg| arg == "--bmap"));
}
