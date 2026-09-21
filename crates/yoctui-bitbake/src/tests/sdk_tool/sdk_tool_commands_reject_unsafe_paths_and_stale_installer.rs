use super::*;

#[test]
fn sdk_tool_commands_reject_unsafe_paths_and_stale_installer() {
    let (directory, adapter) = fixture("unsafe");
    let mut preview = publish_preview(&adapter, &directory);
    preview.argv.push("injected".into());
    assert_eq!(
        adapter.publication_command(&preview),
        Err(SdkToolAdapterError::PreviewMismatch)
    );
    preview.argv.pop();
    fs::write(&preview.request.artifact.path, b"changed installer").unwrap();
    assert!(matches!(
        adapter.publication_command(&preview),
        Err(SdkToolAdapterError::UnsafeInstaller(_))
    ));

    let other = directory.path().join("other-tool");
    executable(&other, "#!/bin/sh\nexit 0\n");
    let mut native = native_preview(&adapter, SdkNativeMode::FindSysroot, None);
    native.request.executable = other.clone();
    native.argv[0] = other;
    assert!(matches!(
        adapter.native_command(&native),
        Err(SdkToolAdapterError::UnsafeTool(_))
    ));

    let destination = directory.path().join("nonempty");
    fs::create_dir(&destination).unwrap();
    fs::write(destination.join("existing"), b"data").unwrap();
    let installer = adapter.sdk_deploy_root.join("fresh.sh");
    fs::write(&installer, b"installer").unwrap();
    let executable = adapter.capability().publish_executable().unwrap();
    let nonempty = SdkPublishPreview::new(executable, artifact(&installer), destination).unwrap();
    assert!(matches!(
        adapter.publication_command(&nonempty),
        Err(SdkToolAdapterError::UnsafeDestination(_))
    ));

    let extracted = directory.path().join("real-extracted");
    fs::create_dir(&extracted).unwrap();
    fs::write(
        extracted.join("environment-setup-core2"),
        "export SDK_ROOT='/opt/sdk'\n",
    )
    .unwrap();
    let linked = directory.path().join("linked-extracted");
    symlink(&extracted, &linked).unwrap();
    let linked_preview = native_preview(&adapter, SdkNativeMode::FindSysroot, Some(linked));
    assert!(matches!(
        adapter.native_command(&linked_preview),
        Err(SdkToolAdapterError::UnsafeExtractedRoot(_))
    ));
}
