use super::*;

#[tokio::test]
async fn pty_sdk_shell_cli_composition_captures_and_routes_persistent_environments() {
    let nonce = std::time::SystemTime::UNIX_EPOCH
        .elapsed()
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "yoctui-cli-pty-sdk-shell-{}-{nonce}",
        std::process::id()
    ));
    for directory in ["source", "build", "sdk"] {
        fs::create_dir_all(root.join(directory)).unwrap();
    }
    let setup = root.join("sdk/environment-setup-x86_64-pokysdk-linux");
    fs::write(
        &setup,
        "export SDKTARGETSYSROOT=/opt/poky/sysroots/target\nexport PATH=/opt/poky/bin:$PATH\n",
    )
    .unwrap();
    let shell = fs::canonicalize("/bin/bash").unwrap();
    let adapter = SdkShellAdapter::default();
    let inspected = adapter
        .inspect("sdk-x86_64".into(), root.join("sdk"), shell.clone())
        .unwrap();
    let sdk = adapter.capture(&inspected).await.unwrap();
    let contexts = PtyContextAuthority::new(
        "workspace".into(),
        root.join("source"),
        root.join("build"),
        VerifiedPtyEnvironment {
            identity: "build-env".into(),
            shell: shell.clone(),
            environment: BTreeMap::from([(
                "BUILDDIR".into(),
                root.join("build").display().to_string(),
            )]),
        },
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![(
            PtyContextEntry {
                identity: sdk.identity.clone(),
                directory: sdk.root.clone(),
            },
            VerifiedPtyEnvironment {
                identity: format!("{}:captured", sdk.identity),
                shell: sdk.shell,
                environment: sdk.environment,
            },
        )],
    )
    .unwrap();
    let router = PtySdkShellRouter::new(contexts);
    let installed = router
        .preview(PtySdkShellAction::InstalledSdk {
            identity: "sdk-x86_64".into(),
        })
        .unwrap();
    assert_eq!(installed.kind, PtySessionKind::SdkShell);
    assert_eq!(
        installed.environment.get("SDKTARGETSYSROOT"),
        Some(&"/opt/poky/sysroots/target".into())
    );
    assert_eq!(installed.command.executable, shell);
    assert_eq!(installed.command.arguments, vec!["-i"]);
    let native = router
        .preview(PtySdkShellAction::NativeBuildEnvironment)
        .unwrap();
    assert_eq!(native.kind, PtySessionKind::NativeShell);
    assert_eq!(native.environment_identity, "build-env");
    fs::remove_dir_all(root).unwrap();
}
