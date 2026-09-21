use super::*;

#[tokio::test]
async fn devtool_metadata_distinguishes_missing_tool_and_workspace_directory() {
    let root = fixture_script("devtool-missing");
    fs::create_dir_all(&root).unwrap();
    let identity = RecipeIdentity {
        name: "busybox".into(),
        file: "/layers/core/busybox_1.0.bb".into(),
    };
    let missing_devtool = root.join("does-not-exist");
    let compatibility = devtool_compatibility(&root, &missing_devtool);
    let missing =
        DevtoolInspector::with_programs(missing_devtool, root.join("does-not-exist-either"))
            .inspect_with_compatibility(&root, identity.clone(), &compatibility, 1)
            .await;
    assert_eq!(missing.capability, DevtoolCapability::MissingExecutable);

    let devtool = root.join("devtool");
    let absent_source = root.join("sources/absent");
    fs::write(
        &devtool,
        format!(
            "#!/bin/sh\nprintf '%s\\n' 'busybox: {}'\n",
            absent_source.display()
        ),
    )
    .unwrap();
    let mut permissions = fs::metadata(&devtool).unwrap().permissions();
    permissions.set_mode(0o700);
    fs::set_permissions(&devtool, permissions).unwrap();
    let compatibility = devtool_compatibility(&root, &devtool);
    let status = DevtoolInspector::with_programs(devtool, root.join("git"))
        .inspect_with_compatibility(&root, identity, &compatibility, 1)
        .await;
    assert_eq!(
        status.workspace,
        DevtoolWorkspace::MissingDirectory {
            source_path: absent_source
        }
    );
    fs::remove_dir_all(root).unwrap();
}
