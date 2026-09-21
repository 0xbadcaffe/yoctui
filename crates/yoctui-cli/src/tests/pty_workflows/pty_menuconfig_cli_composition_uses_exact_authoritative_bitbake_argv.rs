use super::*;

#[test]
fn pty_menuconfig_cli_composition_uses_exact_authoritative_bitbake_argv() {
    let nonce = std::time::SystemTime::UNIX_EPOCH
        .elapsed()
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "yoctui-cli-pty-menuconfig-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(root.join("source")).unwrap();
    fs::create_dir_all(root.join("build")).unwrap();
    let bitbake = root.join("bitbake");
    fs::write(&bitbake, "#!/bin/sh\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&bitbake, fs::Permissions::from_mode(0o700)).unwrap();
    }
    let contexts = PtyContextAuthority::new(
        "workspace".into(),
        root.join("source"),
        root.join("build"),
        VerifiedPtyEnvironment {
            identity: "build-env".into(),
            shell: fs::canonicalize("/bin/sh").unwrap(),
            environment: BTreeMap::new(),
        },
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .unwrap();
    let kernel = RecipeIdentity {
        name: "virtual/kernel".into(),
        file: root.join("source/linux.bb"),
    };
    let router = PtyMenuconfigRouter::new(
        contexts,
        bitbake,
        vec![PtyInteractiveRecipe {
            identity: kernel.clone(),
            tasks: std::collections::BTreeSet::from(["menuconfig".into(), "devshell".into()]),
        }],
        Some(kernel.clone()),
        None,
    )
    .unwrap();
    let menuconfig = router
        .preview(PtyMenuconfigAction::KernelMenuconfig)
        .unwrap();
    assert_eq!(
        menuconfig.command.arguments,
        vec!["-c", "menuconfig", "virtual/kernel"]
    );
    let devshell = router
        .preview(PtyMenuconfigAction::RecipeTask {
            recipe: kernel,
            task: PtyBitBakeInteractiveTask::Devshell,
        })
        .unwrap();
    assert_eq!(devshell.kind, PtySessionKind::Devshell);
    fs::remove_dir_all(root).unwrap();
}
