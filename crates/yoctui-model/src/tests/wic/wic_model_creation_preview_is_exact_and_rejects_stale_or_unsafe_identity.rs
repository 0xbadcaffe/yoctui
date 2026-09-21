use super::*;

#[test]
fn wic_model_creation_preview_is_exact_and_rejects_stale_or_unsafe_identity() {
    let draft = WicCreateDraft {
        machine: "qemux86-64".into(),
        image: "core-image-minimal".into(),
        kickstart: kickstart().identity,
        output_directory: "/build/wic-output".into(),
        generate_bmap: true,
        compression: WicCompression::Xz,
    };
    let preview = draft.preview(&capability()).unwrap();
    assert_eq!(
        preview.argv,
        vec![
            PathBuf::from("/opt/poky/scripts/wic"),
            "create".into(),
            "/layers/meta/wic/directdisk.wks".into(),
            "-e".into(),
            "core-image-minimal".into(),
            "-o".into(),
            "/build/wic-output".into(),
            "--bmap".into(),
            "--compress-with".into(),
            "xz".into(),
        ]
    );
    let mut unsafe_draft = draft.clone();
    unsafe_draft.output_directory = "/build/../escape".into();
    assert!(unsafe_draft.preview(&capability()).is_err());
    let mut stale = capability();
    if let WicCapability::Available { image_targets, .. } = &mut stale {
        image_targets.clear();
    }
    assert!(draft.preview(&stale).is_err());
}
