use super::*;

#[test]
fn qa_layer_capability_and_command_reject_symlink_tampering_and_preview_changes() {
    let (root, snapshot) = fixture("safety", "#!/bin/sh\nexit 0\n");
    let mut exact = preview(&snapshot);
    let command = QaLayerCommandSpec::from_preview(QaLayerSessionId(8), &exact).unwrap();
    assert_eq!(
        command.arguments(),
        &[OsString::from(exact.layer.root.as_os_str())]
    );
    exact.indexed_arguments.push("2: injected".into());
    assert!(matches!(
        QaLayerCommandSpec::from_preview(QaLayerSessionId(8), &exact),
        Err(QaLayerAdapterError::PreviewMismatch)
    ));
    let executable = command.executable().to_owned();
    write_executable(&executable, "#!/bin/sh\nexit 1\n");
    assert!(matches!(
        command.revalidate(),
        Err(QaLayerAdapterError::StaleIdentity(path)) if path == executable
    ));

    let real = root.0.join("real-bin");
    fs::create_dir(&real).unwrap();
    write_executable(&real.join("yocto-check-layer"), "#!/bin/sh\nexit 0\n");
    let link = root.0.join("linked-bin");
    symlink(&real, &link).unwrap();
    let mut limitations = Vec::new();
    assert!(discover_executable(&[link], &mut limitations).is_none());
    assert!(!limitations.is_empty());
}
