use super::*;

#[test]
fn sdk_tool_extracted_environment_is_validated_and_child_only() {
    let (directory, adapter) = fixture("environment");
    let extracted = directory.path().join("extracted");
    fs::create_dir(&extracted).unwrap();
    fs::write(
        extracted.join("environment-setup-core2-64-poky-linux"),
        "export SDK_ROOT='/opt/sdk'\nexport SDK_BIN=\"$SDK_ROOT/bin\"\n",
    )
    .unwrap();
    let preview = native_preview(&adapter, SdkNativeMode::RunNative, Some(extracted.clone()));
    let command = adapter.native_command(&preview).unwrap();
    assert!(command.clears_environment());
    assert_eq!(
        command.environment().get(OsStr::new("SDK_BIN")),
        Some(&OsString::from("/opt/sdk/bin"))
    );
    assert!(!command.environment().contains_key(OsStr::new("HOME")));

    fs::write(
        extracted.join("environment-setup-second"),
        "export SECOND='value'\n",
    )
    .unwrap();
    assert!(matches!(
        adapter.native_command(&preview),
        Err(SdkToolAdapterError::InvalidEnvironment(_))
    ));
}
