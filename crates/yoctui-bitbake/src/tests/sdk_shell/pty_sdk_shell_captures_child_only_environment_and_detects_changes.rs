use super::*;

#[tokio::test]
async fn pty_sdk_shell_captures_child_only_environment_and_detects_changes() {
    let root = std::env::temp_dir().join(format!(
        "yoctui-sdk-shell-{}-{}",
        std::process::id(),
        std::time::SystemTime::UNIX_EPOCH
            .elapsed()
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&root).unwrap();
    let setup = root.join("environment-setup-test");
    fs::write(
        &setup,
        "export YOCTUI_SDK_VALUE=ready\nexport PATH=/sdk/bin:$PATH\n",
    )
    .unwrap();
    let adapter = SdkShellAdapter::default();
    let preview = adapter
        .inspect(
            "sdk-test".into(),
            root.clone(),
            fs::canonicalize("/bin/bash").unwrap(),
        )
        .unwrap();
    let captured = adapter.capture(&preview).await.unwrap();
    assert_eq!(
        captured.environment.get("YOCTUI_SDK_VALUE"),
        Some(&"ready".into())
    );
    assert!(captured.environment["PATH"].starts_with("/sdk/bin:"));
    assert!(std::env::var_os("YOCTUI_SDK_VALUE").is_none());
    fs::write(&setup, "export YOCTUI_SDK_VALUE=changed\n").unwrap();
    assert_eq!(
        adapter.capture(&preview).await,
        Err(SdkShellError::ChangedAfterPreview)
    );
    fs::remove_dir_all(root).unwrap();
}
