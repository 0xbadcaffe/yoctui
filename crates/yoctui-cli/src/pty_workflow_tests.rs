use std::{collections::BTreeMap, fs};

use yoctui_app::{
    PtyBitBakeInteractiveTask, PtyContextAuthority, PtyContextEntry, PtyDevtoolAction,
    PtyDevtoolRouter, PtyInteractiveRecipe, PtyMenuconfigAction, PtyMenuconfigRouter,
    PtySdkShellAction, PtySdkShellRouter, VerifiedPtyEnvironment,
};
use yoctui_bitbake::SdkShellAdapter;
use yoctui_model::{
    DevtoolCapability, DevtoolGitState, DevtoolStatus, DevtoolWorkspace, PtySessionKind,
    RecipeIdentity,
};

#[test]
fn pty_devtool_cli_composition_preserves_exact_interactive_routes() {
    let nonce = std::time::SystemTime::UNIX_EPOCH
        .elapsed()
        .unwrap()
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "yoctui-cli-pty-devtool-{}-{nonce}",
        std::process::id()
    ));
    for path in ["source", "build", "workspace"] {
        fs::create_dir_all(root.join(path)).unwrap();
    }
    let devtool = root.join("devtool");
    fs::write(&devtool, "#!/bin/sh\n").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&devtool, fs::Permissions::from_mode(0o700)).unwrap();
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
        vec![PtyContextEntry {
            identity: "busybox-workspace".into(),
            directory: root.join("workspace"),
        }],
        Vec::new(),
        Vec::new(),
    )
    .unwrap();
    let router = PtyDevtoolRouter::new(contexts, devtool).unwrap();
    let status = DevtoolStatus {
        identity: RecipeIdentity {
            name: "busybox".into(),
            file: root.join("source/busybox.bb"),
        },
        capability: DevtoolCapability::Available,
        workspace: DevtoolWorkspace::Present {
            source_path: root.join("workspace"),
            recipe_file: None,
        },
        git: DevtoolGitState::NotApplicable,
        error: None,
    };
    let shell = router
        .preview(
            &status,
            PtyDevtoolAction::WorkspaceShell {
                workspace_identity: "busybox-workspace".into(),
            },
        )
        .unwrap();
    assert_eq!(shell.kind, PtySessionKind::DevtoolShell);
    let edit = router
        .preview(&status, PtyDevtoolAction::EditRecipe)
        .unwrap();
    assert_eq!(edit.command.arguments, vec!["edit-recipe", "busybox"]);
    assert_eq!(
        router.preview(&status, PtyDevtoolAction::Modify),
        Err(yoctui_app::PtyDevtoolError::UseBackgroundJob)
    );
    fs::remove_dir_all(root).unwrap();
}

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
