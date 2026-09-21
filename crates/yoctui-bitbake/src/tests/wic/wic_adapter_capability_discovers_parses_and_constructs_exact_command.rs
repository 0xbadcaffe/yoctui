use super::*;

#[tokio::test]
async fn wic_adapter_capability_discovers_parses_and_constructs_exact_command() {
    let directory = fixture("exact");
    let program = directory.join("wic");
    executable(
        &program,
        "test \"$1 $2\" = 'list images' && printf 'directdisk  Direct disk\\ncustom Custom\\n'",
    );
    let canned = directory.join("canned");
    fs::create_dir(&canned).unwrap();
    let canned = fs::canonicalize(canned).unwrap();
    fs::write(
        canned.join("directdisk.wks"),
        "part / --source=rootfs --fstype=ext4 --size=64 --align=4\nbootloader --ptable gpt\n",
    )
    .unwrap();
    fs::write(
        canned.join("custom.wks.in"),
        "part /boot --source=bootimg --size=${BOOT_SIZE}\nunsupported value\n",
    )
    .unwrap();
    let capability = WicCapabilityInspector::with_executable(program)
        .with_sources(Vec::new(), vec![canned])
        .inspect(vec!["core-image-minimal".into()])
        .await;
    let WicCapability::Available { kickstarts, .. } = &capability else {
        panic!("available capability: {capability:?}");
    };
    assert_eq!(kickstarts.len(), 2);
    assert_eq!(
        kickstarts[1].partitions[0].mount_point.as_deref(),
        Some("/")
    );
    assert_eq!(kickstarts[1].partitions[0].size_mib, Some(64));
    assert!(
        kickstarts[0]
            .limitations
            .iter()
            .any(|limitation| limitation.contains("dynamic"))
    );

    let output = directory.join("output");
    fs::create_dir(&output).unwrap();
    let output = fs::canonicalize(output).unwrap();
    let draft = WicCreateDraft {
        machine: "qemux86-64".into(),
        image: "core-image-minimal".into(),
        kickstart: kickstarts[1].identity.clone(),
        output_directory: output.display().to_string(),
        generate_bmap: true,
        compression: WicCompression::Gzip,
    };
    let preview = draft.preview(&capability).unwrap();
    let command = WicCreateCommandSpec::from_preview(&preview, &capability).unwrap();
    assert_eq!(
        command.arguments(),
        &[
            OsString::from("create"),
            kickstarts[1]
                .identity
                .path
                .as_ref()
                .unwrap()
                .as_os_str()
                .to_owned(),
            "-e".into(),
            "core-image-minimal".into(),
            "-o".into(),
            output.as_os_str().to_owned(),
            "--bmap".into(),
            "--compress-with".into(),
            "gzip".into(),
        ]
    );
    let alternate = directory.join("alternate-wic");
    executable(&alternate, "exit 0");
    let alternate = fs::canonicalize(alternate).unwrap();
    let mut changed_capability = capability.clone();
    if let WicCapability::Available { executable, .. } = &mut changed_capability {
        *executable = alternate;
    }
    assert_eq!(
        WicCreateCommandSpec::from_preview(&preview, &changed_capability).unwrap_err(),
        WicAdapterError::PreviewMismatch
    );
    let mut tampered = preview;
    tampered.argv.push("--debug".into());
    assert_eq!(
        WicCreateCommandSpec::from_preview(&tampered, &capability).unwrap_err(),
        WicAdapterError::PreviewMismatch
    );
    fs::remove_dir_all(directory).unwrap();
}
