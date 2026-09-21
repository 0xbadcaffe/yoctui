use super::*;

#[test]
fn sdk_tool_commands_reconstruct_exact_publication_and_native_argv() {
    let (directory, adapter) = fixture("commands");
    let publish = publish_preview(&adapter, &directory);
    let publish_command = adapter.publication_command(&publish).unwrap();
    assert_eq!(
        publish_command.arguments(),
        [
            publish.request.artifact.path.as_os_str(),
            publish.request.destination.as_os_str(),
        ]
    );

    let native = native_preview(&adapter, SdkNativeMode::RunNative, None);
    let native_command = adapter.native_command(&native).unwrap();
    assert_eq!(
        native_command.arguments(),
        ["cmake-native", "cmake", "--version"]
    );
    assert_eq!(native_command.current_directory(), adapter.build_directory);
    assert!(!native_command.clears_environment());

    let mut tampered = native;
    tampered.argv.push("injected".into());
    assert_eq!(
        adapter.native_command(&tampered),
        Err(SdkToolAdapterError::PreviewMismatch)
    );
}
