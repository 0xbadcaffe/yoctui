use super::*;

fn inspector_fixture(name: &str) -> (PathBuf, DevtoolInspector, RecipeIdentity) {
    let root = fixture_script(name);
    fs::create_dir_all(&root).unwrap();
    let devtool = root.join("devtool");
    fs::write(
        &devtool,
        "#!/bin/sh\nprintf mutated > conf/bblayers.conf\ntouch spawned\n",
    )
    .unwrap();
    fs::set_permissions(&devtool, fs::Permissions::from_mode(0o700)).unwrap();
    let inspector = DevtoolInspector::with_programs(devtool, root.join("git"));
    let identity = RecipeIdentity {
        name: "busybox".into(),
        file: "/layers/core/busybox.bb".into(),
    };
    (root, inspector, identity)
}

#[tokio::test]
async fn devtool_status_uses_authoritative_bitbake_path_for_unsourced_attach_clients() {
    let (root, inspector, identity) = inspector_fixture("devtool-status-attached-path");
    initialized_devtool_workspace(&root);
    let tools = root.join("vendor-bitbake-bin");
    fs::create_dir(&tools).unwrap();
    let bitbake = tools.join("bitbake");
    fs::write(&bitbake, "#!/bin/sh\nexit 0\n").unwrap();
    fs::set_permissions(&bitbake, fs::Permissions::from_mode(0o700)).unwrap();
    fs::write(root.join("devtool"), format!(
        "#!/bin/sh\n[ \"$1\" = status ] || exit 9\n[ \"$(command -v bitbake)\" = '{}' ] || exit 10\ntouch spawned\n",
        bitbake.display()
    )).unwrap();
    let mut compatibility = devtool_compatibility(&root, &root.join("devtool"));
    let mut tools = compatibility
        .snapshot
        .environment
        .available_tools
        .value()
        .unwrap()
        .clone();
    tools.push(yoctui_model::ToolIdentity {
        id: "bitbake".into(),
        executable: bitbake,
        version: None,
    });
    compatibility.snapshot.environment.available_tools = yoctui_model::AuthoritativeValue::detected(
        tools,
        yoctui_model::IdentityAuthority::InitializedEnvironment,
    );
    let original = fs::read(root.join("conf/bblayers.conf")).unwrap();
    let status = inspector
        .inspect_with_compatibility(&root, identity, &compatibility, 1)
        .await;
    assert_eq!(status.capability, DevtoolCapability::Available);
    assert!(status.error.is_none(), "{:?}", status.error);
    assert!(root.join("spawned").exists());
    assert_eq!(fs::read(root.join("conf/bblayers.conf")).unwrap(), original);
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn devtool_status_preserves_fixed_sdk_status_without_default_workspace_inference() {
    let (root, inspector, identity) = inspector_fixture("devtool-status-fixed-sdk");
    fs::write(root.join(".devtoolbase"), "").unwrap();
    fs::write(
        root.join("devtool"),
        "#!/bin/sh\n[ \"$1\" = status ] || exit 9\ntouch spawned\nexit 0\n",
    )
    .unwrap();
    let compatibility = devtool_compatibility(&root, &root.join("devtool"));
    let status = inspector
        .inspect_with_compatibility(&root, identity, &compatibility, 1)
        .await;
    assert_eq!(status.capability, DevtoolCapability::Available);
    assert!(status.error.is_none());
    assert!(root.join("spawned").exists());
    assert!(!root.join("workspace").exists());
    assert!(!root.join("conf").exists());
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn devtool_status_absent_default_workspace_is_empty_without_initialization() {
    let (root, inspector, identity) = inspector_fixture("devtool-status-absent-workspace");
    fs::create_dir_all(root.join("conf")).unwrap();
    let original = "BBLAYERS ?= \"/layers/core\"\n";
    fs::write(root.join("conf/bblayers.conf"), original).unwrap();
    let compatibility = devtool_compatibility(&root, &root.join("devtool"));
    let status = inspector
        .inspect_with_compatibility(&root, identity, &compatibility, 1)
        .await;
    assert_eq!(status.capability, DevtoolCapability::Available);
    assert_eq!(status.workspace, DevtoolWorkspace::NotMember);
    assert!(status.error.is_none());
    assert!(!root.join("spawned").exists());
    assert!(!root.join("workspace").exists());
    assert_eq!(
        fs::read_to_string(root.join("conf/bblayers.conf")).unwrap(),
        original
    );
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn devtool_status_disabled_ambiguous_and_custom_workspace_never_launch() {
    let (root, inspector, identity) = inspector_fixture("devtool-status-unsafe-workspace");
    initialized_devtool_workspace(&root);
    let workspace = root.join("workspace");
    let cases = [
        "BBLAYERS ?= \"/layers/core\"\n".to_owned(),
        format!(
            "# BBLAYERS = \"{}\"\nBBLAYERS = \"/layers/core\"\n",
            workspace.display()
        ),
        format!(
            "BBLAYERS = \"{}\"\nBBLAYERS = \"/layers/core\"\n",
            workspace.display()
        ),
        format!(
            "BBLAYERS = \"{}\"\nrequire\tother.conf\n",
            workspace.display()
        ),
        format!("BBLAYERS += \"{}\"\n", workspace.display()),
        format!(
            "BBLAYERS = \"${{TOPDIR}}/workspace {}\"\n",
            workspace.display()
        ),
        "x".repeat(65537),
    ];
    let compatibility = devtool_compatibility(&root, &root.join("devtool"));
    for original in cases {
        fs::write(root.join("conf/bblayers.conf"), &original).unwrap();
        let status = inspector
            .inspect_with_compatibility(&root, identity.clone(), &compatibility, 1)
            .await;
        assert!(
            matches!(status.capability, DevtoolCapability::Unavailable { .. }),
            "{original:?}"
        );
        assert!(!root.join("spawned").exists());
        assert_eq!(
            fs::read_to_string(root.join("conf/bblayers.conf")).unwrap(),
            original
        );
    }
    initialized_devtool_workspace(&root);
    fs::write(
        root.join("conf/devtool.conf"),
        "[General]\nworkspace_path=/custom/workspace\n",
    )
    .unwrap();
    let original = fs::read(root.join("conf/bblayers.conf")).unwrap();
    let status = inspector
        .inspect_with_compatibility(&root, identity, &compatibility, 1)
        .await;
    assert!(matches!(
        status.capability,
        DevtoolCapability::Unavailable { .. }
    ));
    assert_eq!(fs::read(root.join("conf/bblayers.conf")).unwrap(), original);
    assert!(!root.join("spawned").exists());
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn devtool_status_enabled_literal_workspace_executes_without_config_changes() {
    let (root, inspector, identity) = inspector_fixture("devtool-status-ready-workspace");
    initialized_devtool_workspace(&root);
    let executable = root.join("devtool");
    fs::write(
        &executable,
        "#!/bin/sh\n[ \"$1\" = status ] || exit 9\n[ \"$BUILDDIR\" = \"$PWD\" ] || exit 10\ntouch spawned\nexit 0\n",
    )
    .unwrap();
    let original = format!(
        "# original\nBBLAYERS ?= \" \\\n  /layers/core \\\n  {} \\\n  \"\n",
        root.join("workspace").display()
    );
    fs::write(root.join("conf/bblayers.conf"), &original).unwrap();
    let compatibility = devtool_compatibility(&root, &executable);
    let status = inspector
        .inspect_with_compatibility(&root, identity, &compatibility, 1)
        .await;
    assert_eq!(status.capability, DevtoolCapability::Available);
    assert!(status.error.is_none());
    assert!(root.join("spawned").exists());
    assert_eq!(
        fs::read_to_string(root.join("conf/bblayers.conf")).unwrap(),
        original
    );
    fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn devtool_status_incomplete_and_nonregular_configuration_fails_closed() {
    let (root, inspector, identity) = inspector_fixture("devtool-status-invalid-config");
    initialized_devtool_workspace(&root);
    let compatibility = devtool_compatibility(&root, &root.join("devtool"));
    fs::remove_file(root.join("workspace/conf/layer.conf")).unwrap();
    let status = inspector
        .inspect_with_compatibility(&root, identity.clone(), &compatibility, 1)
        .await;
    assert!(matches!(
        status.capability,
        DevtoolCapability::Unavailable { .. }
    ));
    initialized_devtool_workspace(&root);
    let configuration = root.join("conf/bblayers.conf");
    fs::write(&configuration, [0xff, 0xfe]).unwrap();
    let status = inspector
        .inspect_with_compatibility(&root, identity.clone(), &compatibility, 1)
        .await;
    assert!(matches!(
        status.capability,
        DevtoolCapability::Unavailable { .. }
    ));
    fs::remove_file(&configuration).unwrap();
    let status = inspector
        .inspect_with_compatibility(&root, identity.clone(), &compatibility, 1)
        .await;
    assert!(matches!(
        status.capability,
        DevtoolCapability::Unavailable { .. }
    ));
    #[cfg(unix)]
    {
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&configuration)
                .status()
                .unwrap()
                .success()
        );
        let status = inspector
            .inspect_with_compatibility(&root, identity, &compatibility, 1)
            .await;
        assert!(matches!(
            status.capability,
            DevtoolCapability::Unavailable { .. }
        ));
    }
    assert!(!root.join("spawned").exists());
    fs::remove_dir_all(root).unwrap();
}
