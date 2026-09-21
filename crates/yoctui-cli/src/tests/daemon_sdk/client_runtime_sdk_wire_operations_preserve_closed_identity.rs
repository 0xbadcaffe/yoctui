use super::*;

#[test]
fn client_runtime_sdk_wire_operations_preserve_closed_identity() {
    let op = DaemonSdkOperation::Native {
        executable: "/sdk/oe-run-native".into(),
        mode: DaemonSdkNativeMode::RunNative,
        extracted_root: None,
        recipe: "cmake-native".into(),
        tool: Some("cmake".into()),
        arguments: vec!["--version".into()],
    };
    let context = DaemonSdkContext {
        build_directory: "/build".into(),
        sdk_deploy_root: "/deploy/sdk".into(),
        workspace_roots: vec!["/sdk".into()],
    };
    let (SdkCommand::Native(preview), cwd) = sdk_command(op, context).unwrap() else {
        panic!()
    };
    assert_eq!(cwd, PathBuf::from("/build"));
    assert_eq!(preview.argv[1], PathBuf::from("cmake-native"));
}
