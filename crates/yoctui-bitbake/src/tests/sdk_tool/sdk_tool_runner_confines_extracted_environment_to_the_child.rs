use super::*;

#[tokio::test]
async fn sdk_tool_runner_confines_extracted_environment_to_the_child() {
    let (directory, adapter) = fixture("child-environment");
    let extracted = directory.path().join("extracted");
    fs::create_dir(&extracted).unwrap();
    fs::write(
        extracted.join("environment-setup-core2-64-poky-linux"),
        "export SDK_ROOT='/opt/sdk'\nexport SDK_BIN=\"$SDK_ROOT/bin\"\n",
    )
    .unwrap();
    let preview = native_preview(&adapter, SdkNativeMode::RunNative, Some(extracted.clone()));
    executable(
        &preview.request.executable,
        "#!/bin/sh\nprintf 'SDK_BIN=%s HOME=%s\\n' \"$SDK_BIN\" \"${HOME-unset}\"\n",
    );
    let preview = SdkNativePreview::new(preview.request).unwrap();
    let command = adapter.native_command(&preview).unwrap();
    let mut runner = SdkToolJobRunner::new();
    runner.start(command).await.unwrap();
    assert_eq!(
        runner.next_event().await.unwrap(),
        SdkToolRunnerEvent::Started
    );
    let output = match runner.next_event().await.unwrap() {
        SdkToolRunnerEvent::Output { line, .. } => line,
        event => panic!("unexpected runner event: {event:?}"),
    };
    assert_eq!(output, "SDK_BIN=/opt/sdk/bin HOME=unset");
    assert!(matches!(
        runner.next_event().await.unwrap(),
        SdkToolRunnerEvent::Completed { exit_code: Some(0) }
    ));
    assert!(std::env::var_os("SDK_BIN").is_none());
}
